import { Plus } from "lucide-react";
import { Scene, StoryEntry } from "../types";
import { TreeItemActions } from "./LibraryActions";
import { useI18n } from "../i18n";

interface Props {
  scenes: Scene[];
  characters: StoryEntry[];
  disabled?: boolean;
  onCreate: () => void;
  onRename: (scene: Scene) => void;
  onDelete: (scene: Scene) => void;
  onMove: (sceneId: string, delta: number) => void;
  onPov: (scene: Scene, povEntryId: string | null) => void;
}

export function SceneStrip({
  scenes,
  characters,
  disabled,
  onCreate,
  onRename,
  onDelete,
  onMove,
  onPov,
}: Props) {
  const { t } = useI18n();
  const ordered = scenes.slice().sort((left, right) => left.position - right.position);
  return (
    <div className="scene-strip">
      <div className="scene-strip-label">{t("scene.label")}</div>
      {ordered.length === 0 && <span className="scene-empty">{t("scene.empty")}</span>}
      {ordered.map((scene, index) => (
        <div key={scene.id} className="scene-chip">
          <span className="scene-index">{index + 1}</span>
          <span className="scene-title">{scene.title}</span>
          {characters.length > 0 && (
            <select
              className="scene-pov"
              value={scene.povEntryId ?? ""}
              title={t("scene.pov")}
              onChange={(event) => onPov(scene, event.target.value || null)}
            >
              <option value="">{t("scene.pov")}</option>
              {characters.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.title}
                </option>
              ))}
            </select>
          )}
          <TreeItemActions
            disableUp={index === 0}
            disableDown={index === ordered.length - 1}
            deleteTitle={t("library.deleteScene")}
            onRename={() => onRename(scene)}
            onDelete={() => onDelete(scene)}
            onMoveUp={() => onMove(scene.id, -1)}
            onMoveDown={() => onMove(scene.id, 1)}
          />
        </div>
      ))}
      <button className="scene-add" disabled={disabled} onClick={onCreate}>
        <Plus size={12} />
        {t("scene.new")}
      </button>
    </div>
  );
}
