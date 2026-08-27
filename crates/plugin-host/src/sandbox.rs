//! 桌面 WASM 沙箱：无 WASI、无宿主导入、有燃料上限。
//! 请求 JSON 写在已有线性内存之后；`plugin_execute` 接受多值返回或 packed i64。
//! Android 不编译本模块，`plugin.operation` 走内置执行器。

use crate::runtime::{PluginInstance, PluginRuntimeError};
use novel_domain::PluginResult;
use serde_json::Value;
use wasmi::{Config, Engine, Linker, Module, Store};

const MAX_FUEL: u64 = 5_000_000;
const PAGE_SIZE: usize = 65_536;
const MAX_PAGES: usize = 16;

pub fn execute(
    instance: &PluginInstance,
    operation: &str,
    input: Value,
) -> Result<PluginResult, PluginRuntimeError> {
    instance.authorize(operation)?;
    let wasm = decode_wasm(instance.manifest.wasm_base64.as_deref())?;
    let request = serde_json::json!({
        "operation": operation,
        "input": input,
    });
    let request_bytes = serde_json::to_vec(&request).map_err(|error| {
        PluginRuntimeError::InvalidOutput(format!("serialize request: {error}"))
    })?;

    let output = run_wasm(&wasm, &request_bytes)?;
    Ok(parse_guest_output(&output))
}

fn decode_wasm(raw: Option<&str>) -> Result<Vec<u8>, PluginRuntimeError> {
    let raw = raw.unwrap_or("").trim();
    if raw.is_empty() {
        return Err(PluginRuntimeError::Sandbox("missing wasm module".into()));
    }
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, raw)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("invalid wasm base64: {error}")))
}

fn run_wasm(wasm: &[u8], request: &[u8]) -> Result<Vec<u8>, PluginRuntimeError> {
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, wasm)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("invalid module: {error}")))?;
    let mut store = Store::new(&engine, ());
    store
        .set_fuel(MAX_FUEL)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("fuel: {error}")))?;
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("instantiate: {error}")))?
        .start(&mut store)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("start: {error}")))?;

    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| PluginRuntimeError::Sandbox("guest must export memory".into()))?;
    // 写到已有线性内存之后，避免盖掉 guest 的 data / 堆。
    let input_ptr = memory.data_size(&store);
    let needed = input_ptr
        .checked_add(request.len())
        .ok_or_else(|| PluginRuntimeError::Sandbox("input overflow".into()))?;
    ensure_capacity(&memory, &mut store, needed)?;
    memory
        .write(&mut store, input_ptr, request)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("write input: {error}")))?;

    let in_ptr = i32::try_from(input_ptr)
        .map_err(|_| PluginRuntimeError::Sandbox("input pointer too large".into()))?;
    let in_len = i32::try_from(request.len())
        .map_err(|_| PluginRuntimeError::Sandbox("input too large".into()))?;
    let (out_ptr, out_len) = call_plugin_execute(&instance, &mut store, in_ptr, in_len)?;
    if out_ptr < 0 || out_len < 0 {
        return Err(PluginRuntimeError::Sandbox(
            "guest returned negative pointer or length".into(),
        ));
    }
    let out_ptr = out_ptr as usize;
    let out_len = out_len as usize;
    let end = out_ptr
        .checked_add(out_len)
        .ok_or_else(|| PluginRuntimeError::Sandbox("output overflow".into()))?;
    if end > memory.data_size(&store) {
        return Err(PluginRuntimeError::Sandbox(
            "guest output is outside memory".into(),
        ));
    }
    let mut output = vec![0u8; out_len];
    memory
        .read(&store, out_ptr, &mut output)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("read output: {error}")))?;
    Ok(output)
}

fn call_plugin_execute(
    instance: &wasmi::Instance,
    store: &mut Store<()>,
    in_ptr: i32,
    in_len: i32,
) -> Result<(i32, i32), PluginRuntimeError> {
    if let Ok(func) = instance.get_typed_func::<(i32, i32), (i32, i32)>(&*store, "plugin_execute") {
        return func
            .call(&mut *store, (in_ptr, in_len))
            .map_err(|error| PluginRuntimeError::Sandbox(format!("trap: {error}")));
    }
    if let Ok(func) = instance.get_typed_func::<(i32, i32), i64>(&*store, "plugin_execute") {
        let packed = func
            .call(&mut *store, (in_ptr, in_len))
            .map_err(|error| PluginRuntimeError::Sandbox(format!("trap: {error}")))?;
        let bits = packed as u64;
        let out_ptr = (bits >> 32) as u32 as i32;
        let out_len = (bits & 0xffff_ffff) as u32 as i32;
        return Ok((out_ptr, out_len));
    }
    Err(PluginRuntimeError::Sandbox(
        "guest must export plugin_execute(i32,i32)->(i32,i32) or plugin_execute(i32,i32)->i64"
            .into(),
    ))
}

fn ensure_capacity(
    memory: &wasmi::Memory,
    store: &mut Store<()>,
    needed: usize,
) -> Result<(), PluginRuntimeError> {
    let current = memory.data_size(&*store);
    if needed <= current {
        return Ok(());
    }
    let extra = needed.saturating_sub(current);
    let pages = extra.div_ceil(PAGE_SIZE);
    let after = current / PAGE_SIZE + pages;
    if after > MAX_PAGES {
        return Err(PluginRuntimeError::Sandbox(
            "guest memory would exceed 16 pages".into(),
        ));
    }
    memory
        .grow(store, pages as u32)
        .map_err(|error| PluginRuntimeError::Sandbox(format!("grow memory: {error}")))?;
    Ok(())
}

fn parse_guest_output(bytes: &[u8]) -> PluginResult {
    let text = String::from_utf8_lossy(bytes);
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => {
            if value.get("output").is_some() {
                PluginResult {
                    output: value.get("output").cloned().unwrap_or(Value::Null),
                    logs: value
                        .get("logs")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|item| item.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["wasm".into()]),
                }
            } else {
                PluginResult {
                    output: value,
                    logs: vec!["wasm".into()],
                }
            }
        }
        Err(_) => PluginResult {
            output: Value::String(text.into_owned()),
            logs: vec!["wasm".into()],
        },
    }
}
