use super::{
    chapter_status, parse_book_id, parse_chapter_id, parse_project_id, parse_rfc3339,
    parse_scene_id, parse_volume_id, SETTING_ACTIVE_PROJECT,
};
use crate::StorageError;
use chrono::Utc;
use novel_domain::{
    Book, BookId, Chapter, ChapterId, ChapterStatus, DomainError, Project, ProjectId, Revision,
    Scene, SceneId, Volume, VolumeId,
};
use rusqlite::{params, OptionalExtension};
use serde_json::json;
use uuid::Uuid;

impl super::Repository {
    pub fn create_project(&self, title: &str) -> Result<Project, StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let project = Project {
            id: ProjectId::new(),
            title: title.to_owned(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.write_with_outbox(
            &project.id.to_string(),
            "project.created",
            json!({ "title": project.title }),
            |tx| {
                tx.execute(
                    "INSERT INTO projects(id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    params![project.id.to_string(), project.title, now, now],
                )?;
                Ok(())
            },
        )?;
        Ok(project)
    }

    pub fn create_book(
        &self,
        project_id: &ProjectId,
        title: &str,
        synopsis: &str,
        position: u32,
    ) -> Result<Book, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM books WHERE project_id = ?1",
                &project_id.to_string(),
            )?
        } else {
            position
        };
        let book = Book {
            id: BookId::new(),
            project_id: project_id.clone(),
            title: title.to_owned(),
            synopsis: synopsis.to_owned(),
            position,
        };
        self.write_with_outbox(
            &project_id.to_string(),
            "book.created",
            json!({
                "bookId": book.id.to_string(),
                "title": book.title,
                "position": position
            }),
            |tx| {
                let inserted = tx.execute(
                    "INSERT INTO books(id, project_id, title, synopsis, position)
                     SELECT ?1, ?2, ?3, ?4, ?5
                     WHERE EXISTS(SELECT 1 FROM projects WHERE id = ?2)",
                    params![
                        book.id.to_string(),
                        project_id.to_string(),
                        book.title,
                        book.synopsis,
                        position
                    ],
                )?;
                if inserted == 0 {
                    return Err(DomainError::NotFound(format!("project {project_id}")).into());
                }
                Ok(())
            },
        )?;
        Ok(book)
    }

    pub fn create_chapter(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Chapter, StorageError> {
        self.create_chapter_with_volume(project_id, book_id, title, position, None)
    }

    pub fn create_chapter_with_volume(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
        volume_id: Option<&str>,
    ) -> Result<Chapter, StorageError> {
        let volume_id = match volume_id {
            Some(value) if !value.is_empty() => Some(parse_volume_id(value)?),
            _ => None,
        };
        let position = if position == 0 {
            let max: i64 = self.connection.query_row(
                "SELECT COALESCE(MAX(position), 0) FROM chapters
                 WHERE book_id = ?1 AND volume_id IS NOT DISTINCT FROM ?2",
                params![book_id, volume_id.as_ref().map(ToString::to_string)],
                |row| row.get(0),
            )?;
            (max as u32).saturating_add(1)
        } else {
            position
        };
        let chapter = Chapter {
            id: ChapterId::new(),
            book_id: Uuid::parse_str(book_id).map(BookId).map_err(|_| {
                StorageError::Domain(DomainError::Validation("invalid book id".into()))
            })?,
            volume_id: volume_id.clone(),
            title: title.to_owned(),
            position,
            current_revision: Revision::INITIAL,
            status: ChapterStatus::Draft,
        };

        self.write_with_outbox(
            &project_id.to_string(),
            "chapter.created",
            json!({
                "chapterId": chapter.id.to_string(),
                "bookId": book_id,
                "volumeId": volume_id.as_ref().map(ToString::to_string),
                "title": title,
                "position": position
            }),
            |tx| {
                let inserted = tx.execute(
                    "INSERT INTO chapters(id, book_id, volume_id, title, position, current_revision, status)
                     SELECT ?1, ?2, ?3, ?4, ?5, 0, 'draft'
                     WHERE EXISTS(SELECT 1 FROM books WHERE id = ?2 AND project_id = ?6)
                       AND (?3 IS NULL OR EXISTS(SELECT 1 FROM volumes WHERE id = ?3 AND book_id = ?2))",
                    params![
                        chapter.id.to_string(),
                        book_id,
                        volume_id.as_ref().map(ToString::to_string),
                        title,
                        position,
                        project_id.to_string()
                    ],
                )?;
                if inserted == 0 {
                    return Err(DomainError::NotFound(format!(
                        "book {book_id} in project {project_id}"
                    ))
                    .into());
                }
                Ok(())
            },
        )?;
        Ok(chapter)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut projects = Vec::new();
        for row in rows {
            let (id, title, created_at, updated_at) = row?;
            projects.push(Project {
                id: parse_project_id(&id)?,
                title,
                created_at: parse_rfc3339(&created_at),
                updated_at: parse_rfc3339(&updated_at),
            });
        }
        Ok(projects)
    }

    pub fn list_books(&self, project_id: &ProjectId) -> Result<Vec<Book>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, project_id, title, synopsis, position
             FROM books WHERE project_id = ?1 ORDER BY position, title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut books = Vec::new();
        for row in rows {
            let (id, project, title, synopsis, position) = row?;
            books.push(Book {
                id: parse_book_id(&id)?,
                project_id: parse_project_id(&project)?,
                title,
                synopsis,
                position: position as u32,
            });
        }
        Ok(books)
    }

    pub fn list_chapters(&self, project_id: &ProjectId) -> Result<Vec<Chapter>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT c.id, c.book_id, c.volume_id, c.title, c.position, c.current_revision, c.status
             FROM chapters c
             JOIN books b ON b.id = c.book_id
             LEFT JOIN volumes v ON v.id = c.volume_id
             WHERE b.project_id = ?1
             ORDER BY b.position, COALESCE(v.position, 2147483647), c.position, c.title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut chapters = Vec::new();
        for row in rows {
            let (id, book_id, volume_id, title, position, revision, status) = row?;
            chapters.push(Chapter {
                id: id.parse().map_err(|_| {
                    StorageError::Domain(DomainError::Validation("invalid chapter id".into()))
                })?,
                book_id: parse_book_id(&book_id)?,
                volume_id: volume_id
                    .map(|value| {
                        value.parse().map_err(|_| {
                            StorageError::Domain(DomainError::Validation(
                                "invalid volume id".into(),
                            ))
                        })
                    })
                    .transpose()?,
                title,
                position: position as u32,
                current_revision: Revision(revision as u64),
                status: chapter_status(&status),
            });
        }
        Ok(chapters)
    }

    pub fn list_volumes(&self, project_id: &ProjectId) -> Result<Vec<Volume>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT v.id, v.book_id, v.title, v.position
             FROM volumes v
             JOIN books b ON b.id = v.book_id
             WHERE b.project_id = ?1
             ORDER BY b.position, v.position, v.title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut volumes = Vec::new();
        for row in rows {
            let (id, book_id, title, position) = row?;
            volumes.push(Volume {
                id: parse_volume_id(&id)?,
                book_id: parse_book_id(&book_id)?,
                title,
                position: position as u32,
            });
        }
        Ok(volumes)
    }

    pub fn create_volume(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Volume, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM volumes WHERE book_id = ?1",
                book_id,
            )?
        } else {
            position
        };
        let volume = Volume {
            id: VolumeId::new(),
            book_id: parse_book_id(book_id)?,
            title: title.to_owned(),
            position,
        };
        self.write_with_outbox(
            &project_id.to_string(),
            "volume.created",
            json!({
                "volumeId": volume.id.to_string(),
                "bookId": book_id,
                "title": title,
                "position": position
            }),
            |tx| {
                let inserted = tx.execute(
                    "INSERT INTO volumes(id, book_id, title, position)
                     SELECT ?1, ?2, ?3, ?4
                     WHERE EXISTS(SELECT 1 FROM books WHERE id = ?2 AND project_id = ?5)",
                    params![
                        volume.id.to_string(),
                        book_id,
                        title,
                        position,
                        project_id.to_string()
                    ],
                )?;
                if inserted == 0 {
                    return Err(DomainError::NotFound(format!(
                        "book {book_id} in project {project_id}"
                    ))
                    .into());
                }
                Ok(())
            },
        )?;
        Ok(volume)
    }

    pub fn rename_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
        title: &str,
    ) -> Result<Volume, StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "volume.renamed",
            json!({ "volumeId": volume_id.to_string(), "title": title }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE volumes SET title = ?2
                     WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?3)",
                    params![volume_id.to_string(), title, project_id.to_string()],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!("volume {volume_id}")).into());
                }
                Ok(())
            },
        )?;
        self.list_volumes(project_id)?
            .into_iter()
            .find(|volume| &volume.id == volume_id)
            .ok_or_else(|| DomainError::NotFound(format!("volume {volume_id}")).into())
    }

    pub fn delete_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
    ) -> Result<(), StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "volume.deleted",
            json!({ "volumeId": volume_id.to_string() }),
            |tx| {
                let deleted = tx.execute(
                    "DELETE FROM volumes
                     WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?2)",
                    params![volume_id.to_string(), project_id.to_string()],
                )?;
                if deleted == 0 {
                    return Err(DomainError::NotFound(format!("volume {volume_id}")).into());
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn move_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
        delta: i32,
    ) -> Result<Vec<Volume>, StorageError> {
        let volumes = self.list_volumes(project_id)?;
        let Some(volume) = volumes.iter().find(|item| &item.id == volume_id) else {
            return Err(DomainError::NotFound(format!("volume {volume_id}")).into());
        };
        let book_id = volume.book_id.clone();
        let mut siblings: Vec<Volume> = volumes
            .into_iter()
            .filter(|item| item.book_id == book_id)
            .collect();
        let Some(index) = siblings.iter().position(|item| &item.id == volume_id) else {
            return Err(DomainError::NotFound(format!("volume {volume_id}")).into());
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < siblings.len() {
            siblings.swap(index, target as usize);
            self.write_with_outbox(
                &project_id.to_string(),
                "volume.reordered",
                json!({ "volumeId": volume_id.to_string(), "delta": delta }),
                |tx| {
                    for (position, item) in siblings.iter().enumerate() {
                        tx.execute(
                            "UPDATE volumes SET position = ?2 WHERE id = ?1",
                            params![item.id.to_string(), (position as u32) + 1],
                        )?;
                    }
                    Ok(())
                },
            )?;
        }
        self.list_volumes(project_id)
    }

    pub fn rename_project(
        &self,
        project_id: &ProjectId,
        title: &str,
    ) -> Result<Project, StorageError> {
        let now = Utc::now().to_rfc3339();
        self.write_with_outbox(
            &project_id.to_string(),
            "project.renamed",
            json!({ "title": title }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE projects SET title = ?2, updated_at = ?3 WHERE id = ?1",
                    params![project_id.to_string(), title, now],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!("project {project_id}")).into());
                }
                Ok(())
            },
        )?;
        self.list_projects()?
            .into_iter()
            .find(|project| &project.id == project_id)
            .ok_or_else(|| DomainError::NotFound(format!("project {project_id}")).into())
    }

    pub fn delete_project(&self, project_id: &ProjectId) -> Result<(), StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "project.deleted",
            json!({}),
            |tx| {
                tx.execute(
                    "DELETE FROM jobs WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM domain_events WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM workflows WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM canon_entities WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM search_documents WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM plot_threads WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM story_entries WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM preference_rules WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM correction_records WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                tx.execute(
                    "DELETE FROM outbox WHERE project_id = ?1",
                    [project_id.to_string()],
                )?;
                let deleted = tx.execute(
                    "DELETE FROM projects WHERE id = ?1",
                    [project_id.to_string()],
                )?;
                if deleted == 0 {
                    return Err(DomainError::NotFound(format!("project {project_id}")).into());
                }
                let active: Option<String> = tx
                    .query_row(
                        "SELECT value FROM app_settings WHERE key = ?1",
                        [SETTING_ACTIVE_PROJECT],
                        |row| row.get(0),
                    )
                    .optional()?;
                if active.as_deref() == Some(project_id.to_string().as_str()) {
                    tx.execute(
                        "DELETE FROM app_settings WHERE key = ?1",
                        [SETTING_ACTIVE_PROJECT],
                    )?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn rename_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        title: &str,
    ) -> Result<Book, StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "book.renamed",
            json!({ "bookId": book_id.to_string(), "title": title }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE books SET title = ?3 WHERE id = ?1 AND project_id = ?2",
                    params![book_id.to_string(), project_id.to_string(), title],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!(
                        "book {book_id} in project {project_id}"
                    ))
                    .into());
                }
                Ok(())
            },
        )?;
        self.list_books(project_id)?
            .into_iter()
            .find(|book| &book.id == book_id)
            .ok_or_else(|| DomainError::NotFound(format!("book {book_id}")).into())
    }

    pub fn delete_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
    ) -> Result<(), StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "book.deleted",
            json!({ "bookId": book_id.to_string() }),
            |tx| {
                let deleted = tx.execute(
                    "DELETE FROM books WHERE id = ?1 AND project_id = ?2",
                    params![book_id.to_string(), project_id.to_string()],
                )?;
                if deleted == 0 {
                    return Err(DomainError::NotFound(format!(
                        "book {book_id} in project {project_id}"
                    ))
                    .into());
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn move_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        delta: i32,
    ) -> Result<Vec<Book>, StorageError> {
        let mut books = self.list_books(project_id)?;
        let Some(index) = books.iter().position(|book| &book.id == book_id) else {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < books.len() {
            books.swap(index, target as usize);
            self.write_with_outbox(
                &project_id.to_string(),
                "book.reordered",
                json!({ "bookId": book_id.to_string(), "delta": delta }),
                |tx| {
                    for (position, book) in books.iter().enumerate() {
                        tx.execute(
                            "UPDATE books SET position = ?2 WHERE id = ?1 AND project_id = ?3",
                            params![
                                book.id.to_string(),
                                (position as u32) + 1,
                                project_id.to_string()
                            ],
                        )?;
                    }
                    Ok(())
                },
            )?;
        }
        self.list_books(project_id)
    }

    pub fn rename_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        title: &str,
    ) -> Result<Chapter, StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "chapter.renamed",
            json!({ "chapterId": chapter_id.to_string(), "title": title }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE chapters SET title = ?2
                     WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?3)",
                    params![chapter_id.to_string(), title, project_id.to_string()],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
                }
                Ok(())
            },
        )?;
        self.list_chapters(project_id)?
            .into_iter()
            .find(|chapter| &chapter.id == chapter_id)
            .ok_or_else(|| DomainError::NotFound(format!("chapter {chapter_id}")).into())
    }

    pub fn delete_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
    ) -> Result<(), StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "chapter.deleted",
            json!({ "chapterId": chapter_id.to_string() }),
            |tx| {
                let deleted = tx.execute(
                    "DELETE FROM chapters
                     WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?2)",
                    params![chapter_id.to_string(), project_id.to_string()],
                )?;
                if deleted == 0 {
                    return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn move_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        delta: i32,
    ) -> Result<Vec<Chapter>, StorageError> {
        let chapters = self.list_chapters(project_id)?;
        let Some(chapter) = chapters.iter().find(|item| &item.id == chapter_id) else {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        };
        let book_id = chapter.book_id.clone();
        let volume_id = chapter.volume_id.clone();
        let mut siblings: Vec<Chapter> = chapters
            .into_iter()
            .filter(|item| item.book_id == book_id && item.volume_id == volume_id)
            .collect();
        let Some(index) = siblings.iter().position(|item| &item.id == chapter_id) else {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < siblings.len() {
            siblings.swap(index, target as usize);
            self.write_with_outbox(
                &project_id.to_string(),
                "chapter.reordered",
                json!({ "chapterId": chapter_id.to_string(), "delta": delta }),
                |tx| {
                    for (position, item) in siblings.iter().enumerate() {
                        tx.execute(
                            "UPDATE chapters SET position = ?2 WHERE id = ?1",
                            params![item.id.to_string(), (position as u32) + 1],
                        )?;
                    }
                    Ok(())
                },
            )?;
        }
        self.list_chapters(project_id)
    }

    pub fn list_scenes(&self, project_id: &ProjectId) -> Result<Vec<Scene>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT s.id, s.chapter_id, s.title, s.position, s.pov_entity_id
             FROM scenes s
             JOIN chapters c ON c.id = s.chapter_id
             JOIN books b ON b.id = c.book_id
             WHERE b.project_id = ?1
             ORDER BY b.position, c.position, s.position, s.title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let mut scenes = Vec::new();
        for row in rows {
            let (id, chapter_id, title, position, pov) = row?;
            scenes.push(Scene {
                id: parse_scene_id(&id)?,
                chapter_id: parse_chapter_id(&chapter_id)?,
                title,
                position: position as u32,
                pov_entry_id: pov.filter(|value| !value.is_empty()),
            });
        }
        Ok(scenes)
    }

    pub fn create_scene(
        &self,
        project_id: &ProjectId,
        chapter_id: &str,
        title: &str,
        position: u32,
        pov_entry_id: Option<&str>,
    ) -> Result<Scene, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM scenes WHERE chapter_id = ?1",
                chapter_id,
            )?
        } else {
            position
        };
        let scene = Scene {
            id: SceneId::new(),
            chapter_id: parse_chapter_id(chapter_id)?,
            title: title.to_owned(),
            position,
            pov_entry_id: pov_entry_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        };
        self.write_with_outbox(
            &project_id.to_string(),
            "scene.created",
            json!({
                "sceneId": scene.id.to_string(),
                "chapterId": chapter_id,
                "title": title,
                "position": position,
                "povEntryId": scene.pov_entry_id,
            }),
            |tx| {
                let inserted = tx.execute(
                    "INSERT INTO scenes(id, chapter_id, title, position, pov_entity_id)
                     SELECT ?1, ?2, ?3, ?4, ?5
                     WHERE EXISTS(
                        SELECT 1 FROM chapters c
                        JOIN books b ON b.id = c.book_id
                        WHERE c.id = ?2 AND b.project_id = ?6
                     )",
                    params![
                        scene.id.to_string(),
                        chapter_id,
                        title,
                        position,
                        scene.pov_entry_id,
                        project_id.to_string()
                    ],
                )?;
                if inserted == 0 {
                    return Err(DomainError::NotFound(format!(
                        "chapter {chapter_id} in project {project_id}"
                    ))
                    .into());
                }
                Ok(())
            },
        )?;
        Ok(scene)
    }

    pub fn rename_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        title: &str,
    ) -> Result<Scene, StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "scene.renamed",
            json!({ "sceneId": scene_id.to_string(), "title": title }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE scenes SET title = ?2
                     WHERE id = ?1 AND chapter_id IN (
                        SELECT c.id FROM chapters c
                        JOIN books b ON b.id = c.book_id
                        WHERE b.project_id = ?3
                     )",
                    params![scene_id.to_string(), title, project_id.to_string()],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!("scene {scene_id}")).into());
                }
                Ok(())
            },
        )?;
        self.list_scenes(project_id)?
            .into_iter()
            .find(|scene| &scene.id == scene_id)
            .ok_or_else(|| DomainError::NotFound(format!("scene {scene_id}")).into())
    }

    pub fn set_scene_pov(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        pov_entry_id: Option<&str>,
    ) -> Result<Scene, StorageError> {
        let pov = pov_entry_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        self.write_with_outbox(
            &project_id.to_string(),
            "scene.updated",
            json!({ "sceneId": scene_id.to_string(), "povEntryId": pov }),
            |tx| {
                let updated = tx.execute(
                    "UPDATE scenes SET pov_entity_id = ?2
                     WHERE id = ?1 AND chapter_id IN (
                        SELECT c.id FROM chapters c
                        JOIN books b ON b.id = c.book_id
                        WHERE b.project_id = ?3
                     )",
                    params![scene_id.to_string(), pov, project_id.to_string()],
                )?;
                if updated == 0 {
                    return Err(DomainError::NotFound(format!("scene {scene_id}")).into());
                }
                Ok(())
            },
        )?;
        self.list_scenes(project_id)?
            .into_iter()
            .find(|scene| &scene.id == scene_id)
            .ok_or_else(|| DomainError::NotFound(format!("scene {scene_id}")).into())
    }

    pub fn delete_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
    ) -> Result<(), StorageError> {
        self.write_with_outbox(
            &project_id.to_string(),
            "scene.deleted",
            json!({ "sceneId": scene_id.to_string() }),
            |tx| {
                let deleted = tx.execute(
                    "DELETE FROM scenes
                     WHERE id = ?1 AND chapter_id IN (
                        SELECT c.id FROM chapters c
                        JOIN books b ON b.id = c.book_id
                        WHERE b.project_id = ?2
                     )",
                    params![scene_id.to_string(), project_id.to_string()],
                )?;
                if deleted == 0 {
                    return Err(DomainError::NotFound(format!("scene {scene_id}")).into());
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn move_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        delta: i32,
    ) -> Result<Vec<Scene>, StorageError> {
        let scenes = self.list_scenes(project_id)?;
        let Some(scene) = scenes.iter().find(|item| &item.id == scene_id) else {
            return Err(DomainError::NotFound(format!("scene {scene_id}")).into());
        };
        let chapter_id = scene.chapter_id.clone();
        let mut siblings: Vec<Scene> = scenes
            .into_iter()
            .filter(|item| item.chapter_id == chapter_id)
            .collect();
        let Some(index) = siblings.iter().position(|item| &item.id == scene_id) else {
            return Err(DomainError::NotFound(format!("scene {scene_id}")).into());
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < siblings.len() {
            siblings.swap(index, target as usize);
            self.write_with_outbox(
                &project_id.to_string(),
                "scene.reordered",
                json!({ "sceneId": scene_id.to_string(), "delta": delta }),
                |tx| {
                    for (position, item) in siblings.iter().enumerate() {
                        tx.execute(
                            "UPDATE scenes SET position = ?2 WHERE id = ?1",
                            params![item.id.to_string(), (position as u32) + 1],
                        )?;
                    }
                    Ok(())
                },
            )?;
        }
        self.list_scenes(project_id)
    }
}
