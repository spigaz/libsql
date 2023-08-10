use crate::{Database, Error, Params, Result, Rows, RowsFuture, Statement, Transaction, TransactionBehavior};

use libsql_sys::ffi;
use std::ffi::c_int;

/// A connection to a libSQL database.
#[derive(Clone, Debug)]
pub struct Connection {
    pub(crate) raw: *mut ffi::sqlite3,
}

// SAFETY: This is safe because we compile sqlite3 w/ SQLITE_THREADSAFE=1
unsafe impl Send for Connection {}

impl Connection {
    /// Connect to the database.
    pub(crate) fn connect(db: &Database) -> Result<Connection> {
        let mut raw = std::ptr::null_mut();
        let db_path = db.db_path.clone();
        let err = unsafe {
            ffi::sqlite3_open_v2(
                std::ffi::CString::new(db_path.as_str())
                    .unwrap()
                    .as_c_str()
                    .as_ptr() as *const _,
                &mut raw,
                ffi::SQLITE_OPEN_READWRITE as c_int | ffi::SQLITE_OPEN_CREATE as c_int,
                std::ptr::null(),
            )
        };
        match err as u32 {
            ffi::SQLITE_OK => {}
            _ => {
                return Err(Error::ConnectionFailed(db_path));
            }
        }
        Ok(Connection { raw })
    }

    /// Get a raw handle to the underlying libSQL connection
    pub fn handle(&self) -> *mut ffi::sqlite3 {
        self.raw
    }

    /// Create a connection from a raw handle to the underlying libSQL connection
    pub fn from_handle(raw: *mut ffi::sqlite3) -> Self {
        Self { raw }
    }

    /// Disconnect from the database.
    pub fn disconnect(&self) {
        unsafe {
            ffi::sqlite3_close_v2(self.raw);
        }
    }

    /// Prepare the SQL statement.
    pub fn prepare<S: Into<String>>(&self, sql: S) -> Result<Statement> {
        Statement::prepare(self.clone(), self.raw, sql.into().as_str())
    }

    pub fn query<S, P>(&self, sql: S, params: P) -> Result<Option<Rows>>
    where
        S: Into<String>,
        P: Into<Params>,
    {
        let stmt = Statement::prepare(self.clone(), self.raw, sql.into().as_str())?;
        let params = params.into();
        let ret = stmt.query(&params)?;
        Ok(Some(ret))
    }

    /// Execute the SQL statement synchronously.
    ///
    /// If you execute a SQL query statement (e.g. `SELECT` statement) that
    /// returns the number of rows changed.
    ///
    /// This method blocks the thread until the SQL statement is executed.
    pub fn execute<S, P>(&self, sql: S, params: P) -> Result<u64>
    where
        S: Into<String>,
        P: Into<Params>,
    {
        let stmt = Statement::prepare(self.clone(), self.raw, sql.into().as_str())?;
        let params = params.into();
        stmt.execute(&params)
    }

    /// Execute the SQL statement synchronously.
    ///
    /// This method never blocks the thread until, but instead returns a
    /// `RowsFuture` object immediately that can be used to deferredly
    /// execute the statement.
    pub fn execute_async<S, P>(&self, sql: S, params: P) -> RowsFuture
    where
        S: Into<String>,
        P: Into<Params>,
    {
        RowsFuture {
            conn: self.clone(),
            sql: sql.into(),
            params: params.into(),
        }
    }

    /// Begin a new transaction in DEFERRED mode, which is the default.
    pub fn transaction(&self) -> Result<Transaction> {
        self.transaction_with_behavior(TransactionBehavior::Deferred)
    }

    /// Begin a new transaction in the given mode.
    pub fn transaction_with_behavior(
        &self,
        tx_behavior: TransactionBehavior,
    ) -> Result<Transaction> {
        Transaction::begin(self.clone(), tx_behavior)
    }

    pub fn is_autocommit(&self) -> bool {
        unsafe { ffi::sqlite3_get_autocommit(self.raw) != 0 }
    }

    pub fn changes(&self) -> u64 {
        unsafe { ffi::sqlite3_changes64(self.raw) as u64 }
    }

    pub fn last_insert_rowid(&self) -> i64 {
        unsafe { ffi::sqlite3_last_insert_rowid(self.raw) }
    }
}
