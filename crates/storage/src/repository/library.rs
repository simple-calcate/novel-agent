use super::{
    chapter_status, parse_book_id, parse_project_id, parse_rfc3339, SETTING_ACTIVE_PROJECT,
};
use crate::StorageError;
use chrono::Utc;
use novel_domain::{
    Book, BookId, Chapter, ChapterId, ChapterStatus, DomainError, Project, ProjectId, Revision,
};
use rusqlite::params;
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
        self.connection.execute(
            "INSERT INTO projects(id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![project.id.to_string(), project.title, now, now],
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
        let inserted = self.connection.execute(
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
        Ok(book)
    }

    pub fn create_chapter(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Chapter, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM chapters WHERE book_id = ?1",
                book_id,
            )?
        } else {
            position
        };
        let chapter = Chapter {
            id: ChapterId::new(),
            book_id: Uuid::parse_str(book_id).map(BookId).map_err(|_| {
                StorageError::Domain(DomainError::Validation("invalid book id".into()))
            })?,
            volume_id: None,
            title: title.to_owned(),
            position,
            current_revision: Revision::INITIAL,
            status: ChapterStatus::Draft,
        };

        let inserted = self.connection.execute(
            "INSERT INTO chapters(id, book_id, title, position, current_revision, status)
             SELECT ?1, ?2, ?3, ?4, 0, 'draft'
             WHERE EXISTS(SELECT 1 FROM books WHERE id = ?2 AND project_id = ?5)",
            params![
                chapter.id.to_string(),
                book_id,
                title,
                position,
                project_id.to_string()
            ],
        )?;
        if inserted == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
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
             WHERE b.project_id = ?1
             ORDER BY b.position, c.position, c.title",
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

    pub fn rename_project(
        &self,
        project_id: &ProjectId,
        title: &str,
    ) -> Result<Project, StorageError> {
        let now = Utc::now().to_rfc3339();
        let updated = self.connection.execute(
            "UPDATE projects SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![project_id.to_string(), title, now],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound(format!("project {project_id}")).into());
        }
        self.list_projects()?
            .into_iter()
            .find(|project| &project.id == project_id)
            .ok_or_else(|| DomainError::NotFound(format!("project {project_id}")).into())
    }

    pub fn delete_project(&self, project_id: &ProjectId) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM jobs WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM domain_events WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM workflows WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM canon_entities WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM search_documents WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        let deleted = self.connection.execute(
            "DELETE FROM projects WHERE id = ?1",
            [project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(DomainError::NotFound(format!("project {project_id}")).into());
        }
        if let Ok(Some(active)) = self.get_setting(SETTING_ACTIVE_PROJECT) {
            if active == project_id.to_string() {
                let _ = self.connection.execute(
                    "DELETE FROM app_settings WHERE key = ?1",
                    [SETTING_ACTIVE_PROJECT],
                );
            }
        }
        Ok(())
    }

    pub fn rename_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        title: &str,
    ) -> Result<Book, StorageError> {
        let updated = self.connection.execute(
            "UPDATE books SET title = ?3 WHERE id = ?1 AND project_id = ?2",
            params![book_id.to_string(), project_id.to_string(), title],
        )?;
        if updated == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
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
        let deleted = self.connection.execute(
            "DELETE FROM books WHERE id = ?1 AND project_id = ?2",
            params![book_id.to_string(), project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
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
            self.write_book_positions(project_id, &books)?;
        }
        self.list_books(project_id)
    }

    pub fn rename_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        title: &str,
    ) -> Result<Chapter, StorageError> {
        let updated = self.connection.execute(
            "UPDATE chapters SET title = ?2
             WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?3)",
            params![chapter_id.to_string(), title, project_id.to_string()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        }
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
        let deleted = self.connection.execute(
            "DELETE FROM chapters
             WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?2)",
            params![chapter_id.to_string(), project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        }
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
        let mut siblings: Vec<Chapter> = chapters
            .into_iter()
            .filter(|item| item.book_id == book_id)
            .collect();
        let Some(index) = siblings.iter().position(|item| &item.id == chapter_id) else {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < siblings.len() {
            siblings.swap(index, target as usize);
            for (position, item) in siblings.iter().enumerate() {
                self.connection.execute(
                    "UPDATE chapters SET position = ?2 WHERE id = ?1",
                    params![item.id.to_string(), (position as u32) + 1],
                )?;
            }
        }
        self.list_chapters(project_id)
    }

    fn write_book_positions(
        &self,
        project_id: &ProjectId,
        books: &[Book],
    ) -> Result<(), StorageError> {
        for (index, book) in books.iter().enumerate() {
            self.connection.execute(
                "UPDATE books SET position = ?2 WHERE id = ?1 AND project_id = ?3",
                params![
                    book.id.to_string(),
                    (index as u32) + 1,
                    project_id.to_string()
                ],
            )?;
        }
        Ok(())
    }
}
