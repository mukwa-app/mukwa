// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Wakunguma Kalimukwa

use crate::{Error, Money, create_test_db};
use jiff::ZonedDateTime;
use jiff::civil::Date;
use rusqlite::{Connection, Row, params};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CategoryGroup {
    pub id: Uuid,
    pub title: String,
    pub kind: GroupKind,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum GroupKind {
    #[default]
    Standard,
    CreditPayment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CategoryKind {
    #[default]
    /// A regular category such as Groceries or Rent.
    Standard,
    /// A category tied to credit account.
    CreditPayment { account_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Category {
    pub id: Uuid,
    pub title: String,
    pub kind: CategoryKind,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Default)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub account_type: AccountType,
}

#[derive(PartialOrd, PartialEq, Debug, Clone, Copy, Ord, Eq, Default)]
pub enum AccountType {
    #[default]
    Cash = 1,
    Credit = 2,
}

#[derive(PartialOrd, PartialEq, Debug, Default, Clone, Copy)]
pub struct CreateBudgetOpts {
    pub amount: Option<Money>,
    /// The budget month, defaults to the current month.
    pub month: Option<Date>,
    pub category_id: Uuid,
}

#[derive(PartialOrd, PartialEq, Debug, Default, Clone, Copy)]
pub struct Budget {
    pub id: Uuid,
    pub amount: Money,
    pub month: i64,
    pub year: i64,
    pub category_id: Uuid,
}

impl<'a> TryFrom<&Row<'a>> for Budget {
    type Error = Error;

    fn try_from(value: &Row<'a>) -> Result<Self, Self::Error> {
        let id: String = value.get("id")?;
        let category_id: String = value.get("category_id")?;
        let amount: i64 = value.get("amount")?;
        let year: i64 = value.get("year")?;
        let month: i64 = value.get("month")?;

        Ok(Budget {
            id: Uuid::parse_str(&id)?,
            year,
            month,
            amount: Money::from_scaled(amount),
            category_id: Uuid::parse_str(&category_id)?,
        })
    }
}

#[derive(PartialOrd, PartialEq, Debug, Clone, Copy, Eq, Ord)]
pub enum TransactionType {
    Expense,
    Income,
    Transfer,
}

#[derive(PartialOrd, PartialEq, Debug, Clone, Copy, Eq, Ord, Default)]
pub enum TransactionKind {
    #[default]
    Regular = 0,
    StartingBalance = 1,
    Correction = 2,
}

impl TryFrom<i64> for TransactionKind {
    type Error = Error;

    fn try_from(v: i64) -> Result<Self, Self::Error> {
        match v {
            x if x == TransactionKind::Regular as i64 => Ok(TransactionKind::Regular),
            x if x == TransactionKind::StartingBalance as i64 => {
                Ok(TransactionKind::StartingBalance)
            }
            x if x == TransactionKind::Correction as i64 => Ok(TransactionKind::Correction),
            _ => Err(Error::new(&format!("Unknown transaction kind: {v}"))),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct Transaction {
    pub id: Uuid,
    /// The sending account.
    pub sender_id: Option<Uuid>,
    /// The receiving account.
    pub receiver_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub date: Date,
    pub note: Option<String>,
    pub amount: Money,
    pub kind: TransactionKind,
    pub audited: bool,
}

impl Transaction {
    pub fn transaction_type(&self) -> TransactionType {
        if self.sender_id.is_some() && self.receiver_id.is_some() {
            return TransactionType::Transfer;
        }

        if self.receiver_id.is_some() {
            return TransactionType::Income;
        }

        TransactionType::Expense
    }
}

impl<'a> TryFrom<&Row<'a>> for Transaction {
    type Error = Error;

    fn try_from(value: &Row<'a>) -> Result<Self, Self::Error> {
        let id: String = value.get("id")?;
        let sender_id: Option<String> = value.get("sender_id")?;
        let receiver_id: Option<String> = value.get("receiver_id")?;
        let category_id: Option<String> = value.get("category_id")?;
        let note: Option<String> = value.get("note")?;
        let transaction_date: String = value.get("transaction_date")?;
        let amount: i64 = value.get("amount")?;
        let kind: TransactionKind = value.get::<_, i64>("kind")?.try_into()?;

        let category_id = match category_id {
            Some(id) => Some(Uuid::parse_str(&id)?),
            None => None,
        };

        let sender_id = match sender_id {
            Some(id) => Some(Uuid::parse_str(&id)?),
            None => None,
        };

        let receiver_id = match receiver_id {
            Some(id) => Some(Uuid::parse_str(&id)?),
            None => None,
        };

        let audited = match value.get::<_, i64>("is_audited")? {
            0 => false,
            1 => true,
            _ => return Err(Error::new("Invalid is_audited value")),
        };

        let transaction = Transaction {
            id: Uuid::parse_str(&id)?,
            amount: Money::from_scaled(amount),
            date: Date::strptime("%Y-%m-%d", transaction_date)?,
            note,
            sender_id,
            receiver_id,
            category_id,
            kind,
            audited,
        };

        Ok(transaction)
    }
}

impl<'a> TryFrom<&Row<'a>> for Account {
    type Error = Error;

    fn try_from(value: &Row<'a>) -> Result<Self, Self::Error> {
        let id: String = value.get("id")?;
        let account_type = match value.get::<_, i64>("account_type_id")? {
            1 => Ok(AccountType::Cash),
            2 => Ok(AccountType::Credit),
            _ => Err(Error::new("Failed to parse account type")),
        };

        let account = Account {
            id: Uuid::parse_str(&id)?,
            name: value.get("name")?,
            account_type: account_type?,
        };

        Ok(account)
    }
}

pub struct Income;
pub struct Expense;
pub struct Transfer;

pub struct TransactionBuilder<T> {
    service: Service,
    amount: Money,
    sender_id: Option<Uuid>,
    receiver_id: Option<Uuid>,
    date: Date,
    category_id: Option<Uuid>,
    note: Option<String>,
    marker: PhantomData<T>,
    kind: TransactionKind,
}

impl<T> TransactionBuilder<T> {
    fn new(service: Service) -> TransactionBuilder<T> {
        Self {
            service,
            amount: Money::ZERO,
            sender_id: None,
            receiver_id: None,
            category_id: None,
            note: None,
            date: ZonedDateTime::now().date(),
            marker: PhantomData,
            kind: TransactionKind::Regular,
        }
    }
}

impl<T> TransactionBuilder<T> {
    /// Sets the transaction amount.
    pub fn amount(mut self, amount: Money) -> TransactionBuilder<T> {
        self.amount = amount;
        self
    }

    /// Sets the transaction note.
    pub fn note(mut self, note: &str) -> TransactionBuilder<T> {
        self.note = Some(note.to_owned());
        self
    }

    /// Sets the transaction date.
    pub fn date(mut self, date: Date) -> TransactionBuilder<T> {
        self.date = date;
        self
    }
}

impl TransactionBuilder<Income> {
    pub fn account(mut self, id: Uuid) -> Self {
        self.receiver_id = Some(id);
        self
    }

    /// Sets the transaction kind
    pub fn kind(mut self, kind: TransactionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Submits the query.
    pub fn submit(self) -> crate::Result<Transaction> {
        let account_id = match self.receiver_id {
            Some(id) => id,
            None => {
                let accounts = self.service.fetch_accounts()?;
                if accounts.is_empty() {
                    return Err(Error::new("Cannot create a transaction without an account"));
                }
                accounts[0].id
            }
        };

        let connection = self.service.connection();
        let sql = "INSERT INTO transactions(id,transaction_date,receiver_id,amount,note,kind) \
            VALUES(?1,?2,?3,?4,?5,?6) \
            RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let params = params![
            Uuid::now_v7().to_string(),
            self.date.to_string(),
            account_id.to_string(),
            self.amount.inner(),
            self.note,
            self.kind as i64
        ];
        let mut rows = stmt.query_and_then(params, |row| Transaction::try_from(row))?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }
}

impl TransactionBuilder<Expense> {
    pub fn account(mut self, id: Uuid) -> Self {
        self.sender_id = Some(id);
        self
    }

    pub fn category(mut self, id: Uuid) -> Self {
        self.category_id = Some(id);
        self
    }

    /// Sets the transaction kind
    pub fn kind(mut self, kind: TransactionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Submits the query.
    pub fn submit(self) -> crate::Result<Transaction> {
        let account_id = match self.sender_id {
            Some(id) => id,
            None => {
                let accounts = self.service.fetch_accounts()?;
                if accounts.is_empty() {
                    return Err(Error::new("Cannot create a transaction without an account"));
                }
                accounts[0].id
            }
        };

        let connection = self.service.connection();
        let sql = "INSERT INTO transactions(id,transaction_date,sender_id,category_id,amount,note,kind) \
            VALUES(?1,?2,?3,?4,?5,?6,?7) \
            RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let params = params![
            Uuid::now_v7().to_string(),
            self.date.to_string(),
            account_id.to_string(),
            self.category_id.map(|c| c.to_string()),
            self.amount.inner(),
            self.note,
            self.kind as i64
        ];
        let mut rows = stmt.query_and_then(params, |row| Transaction::try_from(row))?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }
}
impl TransactionBuilder<Transfer> {
    pub fn accounts(mut self, from: Uuid, to: Uuid) -> Self {
        self.sender_id = Some(from);
        self.receiver_id = Some(to);
        self
    }

    /// Submits the query.
    pub fn submit(self) -> crate::Result<Transaction> {
        let receiver_id = self.receiver_id.ok_or(Error::new("Missing receiver id"))?;
        let sender_id = self.sender_id.ok_or(Error::new("Missing sender id"))?;

        let connection = self.service.connection();
        let sql = "INSERT INTO transactions(id,transaction_date,receiver_id,sender_id,amount,note) \
            VALUES(?1,?2,?3,?4,?5,?6) \
            RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let params = params![
            Uuid::now_v7().to_string(),
            self.date.to_string(),
            receiver_id.to_string(),
            sender_id.to_string(),
            self.amount.inner(),
            self.note,
        ];
        let mut rows = stmt.query_and_then(params, |row| Transaction::try_from(row))?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }
}

#[derive(Clone)]
pub struct Service {
    connection: Rc<Connection>,
}

impl Service {
    pub fn new(connection: Connection) -> Service {
        Service {
            connection: Rc::new(connection),
        }
    }

    pub fn connection(&self) -> Rc<Connection> {
        self.connection.clone()
    }

    pub fn vacuum(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let path = path.as_ref();
        let connection = self.connection();
        connection.execute("VACUUM main INTO ?", [path.to_str().unwrap()])?;
        Ok(())
    }

    pub fn open(path: impl AsRef<Path>) -> crate::Result<Service> {
        let connection = Connection::open(path)?;
        let service = Service {
            connection: Rc::new(connection),
        };

        Ok(service)
    }

    /// Opens an in-memory service.
    pub fn open_in_memory() -> crate::Result<Service> {
        let connection = create_test_db();
        let service = Service {
            connection: Rc::new(connection),
        };

        service.create_account("Test account", AccountType::Cash)?;

        Ok(service)
    }

    /// Fetches all accounts from the database.
    pub fn fetch_accounts(&self) -> crate::Result<Vec<Account>> {
        let mut accounts = vec![];
        let connection = self.connection();
        let sql = "SELECT * FROM accounts";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then([], |row| Account::try_from(row))?;

        for row in rows {
            accounts.push(row?);
        }
        Ok(accounts)
    }

    /// Creates a new expense builder.
    ///
    /// ## Example
    /// ```
    /// use mukwa_core::Money;
    /// use mukwa_core::service::{TransactionType,Service,AccountType};
    ///
    /// fn main() -> mukwa_core::Result<()>{
    ///     let service = Service::open_in_memory()?;
    ///
    ///     let account = service.create_account("Credit card",AccountType::Credit)?;
    ///     let transaction = service
    ///         .create_expense()
    ///         .amount(Money::new(5))
    ///         .account(account.id)
    ///         .submit()?;
    ///
    ///     assert_eq!(transaction.amount,Money::new(5));
    ///     assert_eq!(transaction.transaction_type(),TransactionType::Expense);
    ///     Ok(())
    /// }
    /// ```
    pub fn create_expense(&self) -> TransactionBuilder<Expense> {
        TransactionBuilder::new(self.clone())
    }

    /// Creates a new income builder.
    ///
    /// ## Example
    /// ```
    /// use mukwa_core::Money;
    /// use jiff::civil::date;
    /// use mukwa_core::service::{TransactionType,AccountType,Service};
    ///
    /// fn main() -> mukwa_core::Result<()>{
    ///     let service = Service::open_in_memory()?;
    ///
    ///     let account = service.create_account("Chequing",AccountType::Cash)?;
    ///     let transaction = service
    ///         .create_income()
    ///         .amount(Money::new(12_500))
    ///         .account(account.id)
    ///         .date(date(2020,1,1))
    ///         .submit()?;
    ///
    ///     assert_eq!(transaction.amount,Money::new(12_500));
    ///     assert_eq!(transaction.date,date(2020,1,1));
    ///     assert_eq!(transaction.transaction_type(),TransactionType::Income);
    ///     Ok(())
    /// }
    /// ```
    pub fn create_income(&self) -> TransactionBuilder<Income> {
        TransactionBuilder::new(self.clone())
    }

    /// Creates a new transfer builder.
    ///
    /// ## Example
    /// ```
    /// use mukwa_core::Money;
    /// use mukwa_core::service::{TransactionType,AccountType,Service};
    ///
    /// fn main() -> mukwa_core::Result<()>{
    ///     let service = Service::open_in_memory()?;
    ///
    ///     let account = service.create_account("Chequing",AccountType::Cash)?;
    ///     let account2 = service.create_account("Savings",AccountType::Cash)?;
    ///     let transaction = service
    ///         .create_transfer()
    ///         .amount(Money::new(4000))
    ///         .accounts(account.id,account2.id)
    ///         .submit()?;
    ///
    ///     assert_eq!(transaction.transaction_type(),TransactionType::Transfer);
    ///     Ok(())
    /// }
    /// ```
    pub fn create_transfer(&self) -> TransactionBuilder<Transfer> {
        TransactionBuilder::new(self.clone())
    }

    /// Fetches all the budgets in a specific month.
    pub fn fetch_budgets_by_month(&self, date: Date) -> crate::Result<Vec<Budget>> {
        let mut budgets = vec![];
        let connection = self.connection();
        let sql = "SELECT * FROM budgets WHERE month = ?1 AND year = ?2";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then(params![date.month(), date.year()], |row| {
            Budget::try_from(row)
        })?;

        for row in rows {
            budgets.push(row?);
        }
        Ok(budgets)
    }

    /// Fetches all the budgets.
    pub fn fetch_budgets(&self) -> crate::Result<Vec<Budget>> {
        let mut budgets = vec![];
        let connection = self.connection();
        let sql = "SELECT * FROM budgets";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then((), |row| Budget::try_from(row))?;

        for row in rows {
            budgets.push(row?);
        }
        Ok(budgets)
    }

    pub fn fetch_or_init_budgets(&self, date: Date) -> crate::Result<Vec<Budget>> {
        let groups = self.fetch_categories()?;
        let budgets = self.fetch_budgets_by_month(date)?;
        let categories = groups.into_iter().flat_map(|g| g.categories);

        for category in categories {
            if budgets
                .iter()
                .find(|budget| budget.category_id == category.id)
                .is_some()
            {
                continue;
            }
            self.create_budget(CreateBudgetOpts {
                category_id: category.id,
                month: Some(date),
                ..Default::default()
            })?;
        }

        self.fetch_budgets_by_month(date)
    }

    /// Deletes a category from the database
    pub fn delete_category(&self, id: Uuid) -> crate::Result<()> {
        let category = self.get_category(id)?;
        if matches!(category.kind, CategoryKind::CreditPayment { account_id: _ }) {
            return Err(Error::new("Meta categories cannot be deleted"));
        }

        let connection = self.connection();
        let sql = "DELETE FROM categories WHERE id = ?";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([id.to_string()])?;
        Ok(())
    }

    /// Deletes a category group from the database
    pub fn delete_category_group(&self, id: Uuid) -> crate::Result<()> {
        let group = self.get_category_group(id)?;
        if group.kind == GroupKind::CreditPayment {
            return Err(Error::new("Meta category groups cannot be deleted"));
        }

        let connection = self.connection();
        let sql = "DELETE FROM category_groups WHERE id = ?";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([id.to_string()])?;
        Ok(())
    }

    /// Creates a correction transaction that makes the account balance match `balance`. E.g. if
    /// `account_id` has a balance of 50 and `balance = 75` then a correction of `25` will be made.
    pub fn create_correction_transaction(
        &self,
        account_id: Uuid,
        balance: Money,
    ) -> crate::Result<Transaction> {
        // TODO: prevent setting payee on other kinds of transactions
        let account_balance = self.account_balance(account_id)?;
        if balance == account_balance {
            return Err(Error::new(
                "Cannot create correction when account balances are equal",
            ));
        }

        let connection = self.connection();
        let sql = if balance > account_balance {
            "INSERT INTO transactions(id,transaction_date,receiver_id,amount,kind) \
            VALUES(?1,?2,?3,?4,?5) \
            RETURNING *"
        } else {
            "INSERT INTO transactions(id,transaction_date,sender_id,amount,kind) \
            VALUES(?1,?2,?3,?4,?5) \
            RETURNING *"
        };

        let correction = (balance - account_balance).abs();
        let mut stmt = connection.prepare_cached(sql)?;
        let params = params![
            Uuid::now_v7().to_string(),
            ZonedDateTime::now().date().to_string(),
            account_id.to_string(),
            correction.inner(),
            TransactionKind::Correction as i64
        ];
        let mut rows = stmt.query_and_then(params, |row| Transaction::try_from(row))?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    /// Fetches all transactions from the database.
    pub fn fetch_transactions(&self) -> crate::Result<Vec<Transaction>> {
        let mut transactions = vec![];
        let connection = self.connection();
        let sql = "SELECT * FROM transactions";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then([], |row| Transaction::try_from(row))?;

        for row in rows {
            transactions.push(row?);
        }
        Ok(transactions)
    }

    fn credit_payments(&self, account_id: Uuid, month: Date) -> crate::Result<Money> {
        let transactions = self.fetch_transactions()?;
        let total: Money = transactions
            .iter()
            .filter(|t| t.transaction_type() == TransactionType::Transfer)
            .filter(|t| t.date.month() == month.month() && t.date.year() == month.year())
            .filter(|t| t.sender_id.is_some() && t.receiver_id.unwrap_or_default() == account_id)
            .map(|t| t.amount)
            .sum();

        Ok(total)
    }

    /// Calculates the total amount spent in the category in a specific month.
    pub fn total_spent(&self, category_id: Uuid, month: Date) -> crate::Result<Money> {
        let category = self.get_category(category_id)?;
        if let CategoryKind::CreditPayment { account_id } = category.kind {
            return self.credit_payments(account_id, month);
        }

        let connection = self.connection();
        let sql = "SELECT * FROM transactions WHERE category_id = ?1";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows =
            stmt.query_and_then([category_id.to_string()], |row| Transaction::try_from(row))?;

        let mut total = Money::ZERO;
        for row in rows {
            let transaction = row?;
            let is_same_month = transaction.date.month() == month.month()
                && transaction.date.year() == month.year();

            if !is_same_month || transaction.kind == TransactionKind::StartingBalance {
                continue;
            }

            total += transaction.amount;
        }
        Ok(total)
    }

    /// Creates a monthly target amount for a category.
    pub fn create_budget_target(&self, category_id: Uuid, amount: Money) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "INSERT INTO budget_targets(category_id,amount) VALUES(?1,?2)";
        connection.execute(sql, params![category_id.to_string(), amount.inner()])?;
        Ok(())
    }

    pub fn get_budget_target(&self, category_id: Uuid) -> crate::Result<Money> {
        let connection = self.connection();
        let sql = "SELECT amount from budget_targets WHERE category_id = ?";
        let amount = connection.query_one(sql, [category_id.to_string()], |row| {
            row.get::<_, i64>(0).map(Money::from_scaled)
        })?;
        Ok(amount)
    }

    pub fn update_budget_target(&self, category_id: Uuid, amount: Money) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "UPDATE budget_targets SET amount = ?1 WHERE category_id = ?2";
        connection.execute(sql, params![amount.inner(), category_id.to_string(),])?;
        Ok(())
    }

    pub fn delete_budget_target(&self, category_id: Uuid) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "DELETE FROM budget_targets WHERE category_id = ?";
        connection.execute(sql, [category_id.to_string()])?;
        Ok(())
    }

    /// Calculates the total amount spent in the category group.
    pub fn total_spent_in_group(&self, group_id: Uuid, month: Date) -> crate::Result<Money> {
        let connection = self.connection();

        let sql = "SELECT \
            t.id,t.amount,t.transaction_date,t.sender_id,t.receiver_id,t.category_id,t.note,t.kind,t.is_audited \
            FROM transactions t \
            LEFT JOIN categories c ON t.category_id = c.id \
            WHERE c.group_id = ?";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then([group_id.to_string()], |row| Transaction::try_from(row))?;

        let mut total = Money::ZERO;
        for row in rows {
            let transaction = row?;

            let is_same_month = transaction.date.month() == month.month()
                && transaction.date.year() == month.year();

            if !is_same_month {
                continue;
            }

            total += transaction.amount;
        }
        Ok(total)
    }

    /// Sets the `is_audited` field of all the account's transactions to `true`.
    pub fn audit_account(&self, account_id: Uuid) -> crate::Result<()> {
        let sql =
            "UPDATE transactions SET is_audited = TRUE WHERE sender_id = ?1 OR receiver_id = ?1";
        let connection = self.connection();
        connection.execute(sql, [account_id.to_string()])?;
        let transactions = self.fetch_transactions()?;
        let ids = transactions
            .iter()
            .filter(|t| t.receiver_id == Some(account_id) || t.sender_id == Some(account_id))
            .map(|t| t.id);

        for id in ids {
            self.confirm_transaction(id)?;
        }
        Ok(())
    }

    /// Sets the `is_audited` field of all the account's transactions to `false`.
    pub fn unlock_all_transactions(&self, account_id: Uuid) -> crate::Result<()> {
        let sql =
            "UPDATE transactions SET is_audited = FALSE WHERE sender_id = ?1 OR receiver_id = ?1";
        let connection = self.connection();
        connection.execute(sql, [account_id.to_string()])?;
        Ok(())
    }

    pub fn total_assigned_in_group(&self, group_id: Uuid, month: Date) -> crate::Result<Money> {
        let connection = self.connection();

        let sql = "SELECT b.* FROM budgets b \
            LEFT JOIN categories c ON b.category_id = c.id \
            WHERE c.group_id = ?";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then([group_id.to_string()], |row| Budget::try_from(row))?;

        let mut total = Money::ZERO;
        for row in rows {
            let budget = row?;

            let is_same_month =
                budget.month == month.month() as i64 && budget.year == month.year() as i64;

            if !is_same_month {
                continue;
            }

            total += budget.amount;
        }
        Ok(total)
    }

    /// Calculates the account balance.
    pub fn account_balance(&self, account_id: Uuid) -> crate::Result<Money> {
        let connection = self.connection();
        let mut expense_stmt =
            connection.prepare_cached("SELECT amount FROM transactions WHERE sender_id = ?1")?;
        let mut income_stmt =
            connection.prepare_cached("SELECT amount FROM transactions WHERE receiver_id = ?1")?;
        let total_expenses: Money = expense_stmt
            .query_and_then([account_id.to_string()], |row| {
                let amount = row.get::<_, i64>("amount")?;
                Ok(Money::from_scaled(amount))
            })?
            .collect::<Result<Vec<_>, Error>>()?
            .iter()
            .copied()
            .sum();

        let total_income: Money = income_stmt
            .query_and_then([account_id.to_string()], |row| {
                let amount = row.get::<_, i64>("amount")?;
                Ok(Money::from_scaled(amount))
            })?
            .collect::<Result<Vec<_>, Error>>()?
            .iter()
            .copied()
            .sum();

        let total = total_income - total_expenses;
        Ok(total)
    }

    /// Fetches all categories from the database.
    pub fn fetch_categories(&self) -> crate::Result<Vec<CategoryGroup>> {
        let mut groups = vec![];
        let connection = self.connection();
        let mut group_stmt = connection.prepare_cached("SELECT * FROM category_groups")?;
        let mut group_rows = group_stmt.query(())?;

        while let Ok(Some(row)) = group_rows.next() {
            let id: String = row.get("id")?;
            let title: String = row.get("title")?;
            let is_meta: bool = row.get("is_meta")?;

            let kind = if is_meta {
                GroupKind::CreditPayment
            } else {
                GroupKind::Standard
            };

            let group = CategoryGroup {
                id: Uuid::parse_str(&id)?,
                title,
                kind,
                categories: Vec::new(),
            };
            groups.push(group);
        }

        let mut category_stmt = connection.prepare_cached("SELECT * FROM categories")?;
        let mut category_rows = category_stmt.query(())?;
        while let Ok(Some(row)) = category_rows.next() {
            let id: String = row.get("id")?;
            let title: String = row.get("title")?;
            let group_id = row
                .get::<_, String>("group_id")
                .map(|id| Uuid::parse_str(&id).unwrap())?;
            let account_id = row
                .get::<_, Option<String>>("account_id")?
                .map(|id| Uuid::parse_str(&id).unwrap());

            let kind = match account_id {
                Some(id) => CategoryKind::CreditPayment { account_id: id },
                None => CategoryKind::Standard,
            };

            let category = Category {
                id: Uuid::parse_str(&id)?,
                title,
                kind,
            };

            if let Some(group) = groups.iter_mut().find(|g| g.id == group_id) {
                group.categories.push(category);
            }
        }
        Ok(groups)
    }

    /// Fetches all the categories in the category group.
    pub fn fetch_categories_in_group(&self, id: Uuid) -> crate::Result<Vec<Category>> {
        // let mut categories = vec![];
        // let connection = self.connection();
        // let sql = "SELECT * FROM categories WHERE group_id = ?";
        // let mut stmt = connection.prepare_cached(sql)?;
        // let rows = stmt.query_and_then([id.to_string()], |row| Category::try_from(row))?;

        // for row in rows {
        //     categories.push(row?);
        // }
        // Ok(categories)
        todo!()
    }

    /// Fetches all category groups from the database.
    #[deprecated = "use Service::fetch_categories instead"]
    pub fn fetch_category_groups(&self) -> crate::Result<Vec<CategoryGroup>> {
        // let mut category_groups = vec![];
        // let connection = self.connection();
        // let sql = "SELECT * FROM category_groups";
        // let mut stmt = connection.prepare_cached(sql)?;
        // let rows = stmt.query_and_then([], |row| CategoryGroup::try_from(row))?;

        // for row in rows {
        //     category_groups.push(row?);
        // }
        // Ok(category_groups)
        todo!()
    }

    pub fn fetch_unconfirmed_transactions(&self) -> crate::Result<Vec<Uuid>> {
        let connection = self.connection();
        let sql = "SELECT transaction_id FROM unconfirmed_transactions";
        let mut stmt = connection.prepare_cached(sql)?;
        let rows = stmt.query_and_then([], |row| {
            let id: String = row.get(0)?;
            let id = Uuid::parse_str(&id)?;
            Ok::<_, Error>(id)
        })?;

        let mut ids = vec![];
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    /// Creates a new [`Account`].
    pub fn create_account(&self, name: &str, account_type: AccountType) -> crate::Result<Account> {
        let account_type_id = match account_type {
            AccountType::Cash => 1,
            AccountType::Credit => 2,
        };
        let connection = self.connection();
        let sql = "INSERT INTO accounts(id,name,account_type_id) VALUES(?1,?2,?3) RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt.query_and_then(
            params![&Uuid::now_v7().to_string(), name, account_type_id],
            |row| Account::try_from(row),
        )?;
        let account = rows.next().unwrap()?;

        if account_type == AccountType::Credit {
            let groups = self.fetch_categories()?;
            let credit_group = groups
                .iter()
                .find(|group| group.kind == GroupKind::CreditPayment)
                .ok_or(Error::new("Credit payments group does not exist"))?;

            let connection = self.connection();
            let sql = "INSERT INTO categories(id,title,group_id,account_id) VALUES(?1,?2,?3,?4) RETURNING id";
            let mut stmt = connection.prepare_cached(sql)?;
            let category_id: String = stmt.query_one(
                params![
                    Uuid::now_v7().to_string(),
                    account.name.clone(),
                    credit_group.id.to_string(),
                    account.id.to_string()
                ],
                |row| row.get(0),
            )?;

            let date = ZonedDateTime::now().date();
            let sql = "INSERT INTO budgets(id,amount,category_id,month,year) \
            VALUES(?1,?2,?3,?4,?5)";

            let mut stmt = connection.prepare_cached(sql)?;
            let params = params![
                Uuid::now_v7().to_string(),
                Money::ZERO.inner(),
                category_id,
                date.month(),
                date.year()
            ];
            stmt.execute(params)?;
        }
        Ok(account)
    }

    /// Creates a new [`Category`].
    pub fn create_category(&self, title: &str, group_id: Uuid) -> crate::Result<Category> {
        let connection = self.connection();
        let id = Uuid::now_v7();
        let sql = "INSERT INTO categories(id,title,group_id) VALUES(?1,?2,?3)";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute(params![&id.to_string(), title, group_id.to_string()])?;
        self.get_category(id)
    }

    /// Marks a transaction as unconfirmed.
    pub fn mark_as_unconfirmed(&self, transaction_id: Uuid) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "INSERT INTO unconfirmed_transactions(transaction_id) VALUES(?)";
        let mut stmt = connection.prepare_cached(sql)?;

        stmt.execute([transaction_id.to_string()])?;
        Ok(())
    }

    /// Marks a transaction as confirmed.
    pub fn confirm_transaction(&self, transaction_id: Uuid) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "DELETE FROM unconfirmed_transactions WHERE transaction_id = ?";
        let mut stmt = connection.prepare_cached(sql)?;

        stmt.execute([transaction_id.to_string()])?;
        Ok(())
    }

    /// Creates a new [`CategoryGroup`].
    pub fn create_category_group(&self, title: &str) -> crate::Result<CategoryGroup> {
        let connection = self.connection();
        let id = Uuid::now_v7();
        let sql = "INSERT INTO category_groups(id,title) VALUES(?1,?2)";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([&id.to_string(), title])?;
        self.get_category_group(id)
    }

    /// Creates a new [`Budget`].
    pub fn create_budget(&self, opts: CreateBudgetOpts) -> crate::Result<Budget> {
        let connection = self.connection();
        let amount = opts.amount.unwrap_or_default().inner();
        let date = opts.month.unwrap_or(ZonedDateTime::now().date());

        let sql = "INSERT INTO budgets(id,amount,category_id,month,year) \
        VALUES(?1,?2,?3,?4,?5) \
        RETURNING *";

        let mut stmt = connection.prepare_cached(sql)?;
        let params = params![
            Uuid::now_v7().to_string(),
            amount,
            opts.category_id.to_string(),
            date.month(),
            date.year()
        ];
        let mut rows = stmt.query_and_then(params, |row| Budget::try_from(row))?;
        let budget = rows.next().unwrap()?;

        Ok(budget)
    }

    /// Updates the category title.
    pub fn update_category(&self, id: Uuid, title: &str) -> crate::Result<Category> {
        let connection = self.connection();
        let sql = "UPDATE categories SET title = ?1 WHERE id = ?2";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([title, id.to_string().as_str()])?;
        self.get_category(id)
    }

    /// Moves the category into the category group.
    pub fn move_category(&self, id: Uuid, group_id: Uuid) -> crate::Result<()> {
        let connection = self.connection();
        let sql = "UPDATE categories SET group_id = ?1 WHERE id = ?2";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([group_id.to_string().as_str(), id.to_string().as_str()])?;
        Ok(())
    }

    /// Updates the category group title.
    pub fn update_category_group(&self, id: Uuid, title: &str) -> crate::Result<CategoryGroup> {
        let connection = self.connection();
        let sql = "UPDATE category_groups SET title = ?1 WHERE id = ?2";
        let mut stmt = connection.prepare_cached(sql)?;
        stmt.execute([title, id.to_string().as_str()])?;
        self.get_category_group(id)
    }

    /// Updates the budget amount.
    pub fn update_budget(&self, id: Uuid, amount: Money) -> crate::Result<Budget> {
        let connection = self.connection();
        let sql = "UPDATE budgets SET amount = ?1 WHERE id = ?2 RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt
            .query_and_then(params![amount.inner(), id.to_string().as_str()], |row| {
                Budget::try_from(row)
            })?;
        let budget = rows.next().unwrap()?;
        Ok(budget)
    }

    pub fn get_transaction(&self, id: Uuid) -> crate::Result<Transaction> {
        let connection = self.connection();
        let mut stmt = connection.prepare_cached("SELECT * FROM transactions WHERE id = ?")?;
        let mut rows = stmt.query_and_then([id.to_string()], |row| Transaction::try_from(row))?;
        rows.next().ok_or(Error::new("Transaction not found"))?
    }

    pub fn get_account(&self, id: Uuid) -> crate::Result<Account> {
        let connection = self.connection();
        let mut stmt = connection.prepare_cached("SELECT * FROM accounts WHERE id = ?")?;
        let mut rows = stmt.query_and_then([id.to_string()], |row| Account::try_from(row))?;
        rows.next().ok_or(Error::new("Account not found"))?
    }

    pub fn get_category_group(&self, id: Uuid) -> crate::Result<CategoryGroup> {
        let groups = self.fetch_categories()?;
        groups
            .into_iter()
            .find(|g| g.id == id)
            .ok_or(Error::new("Category group not found"))
    }

    pub fn get_category(&self, id: Uuid) -> crate::Result<Category> {
        let groups = self.fetch_categories()?;
        for group in groups {
            for category in group.categories {
                if category.id == id {
                    return Ok(category);
                }
            }
        }
        Err(Error::new("Category not found"))
    }

    pub fn get_budget(&self, id: Uuid) -> crate::Result<Budget> {
        let connection = self.connection();
        let mut stmt = connection.prepare_cached("SELECT * FROM budgets WHERE id = ?")?;
        let mut rows = stmt.query_and_then([id.to_string()], |row| Budget::try_from(row))?;
        rows.next().ok_or(Error::new("Budget not found"))?
    }

    pub fn set_transaction_note(&self, id: Uuid, note: &str) -> crate::Result<Transaction> {
        if self.get_transaction(id)?.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }
        let connection = self.connection();
        let sql = "UPDATE transactions SET note = ?1 WHERE id = ?2 RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt.query_and_then(params![note, id.to_string().as_str()], |row| {
            Transaction::try_from(row)
        })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    pub fn set_transaction_date(&self, id: Uuid, date: Date) -> crate::Result<Transaction> {
        if self.get_transaction(id)?.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }
        let connection = self.connection();
        let sql = "UPDATE transactions SET transaction_date = ?1 WHERE id = ?2 RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt
            .query_and_then(params![date.to_string(), id.to_string().as_str()], |row| {
                Transaction::try_from(row)
            })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    pub fn set_transaction_category(
        &self,
        id: Uuid,
        category_id: Uuid,
    ) -> crate::Result<Transaction> {
        let category = self.get_category(category_id)?;
        if matches!(category.kind, CategoryKind::CreditPayment { account_id: _ }) {
            let error = Error::new("Cannot assign meta category to transaction");
            return Err(error);
        }

        let transaction = self.get_transaction(id)?;
        if transaction.transaction_type() != TransactionType::Expense {
            let error = Error::new("Invalid transaction type (only expenses can have a category)");
            return Err(error);
        }

        if transaction.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }

        let connection = self.connection();
        let sql = "UPDATE transactions SET category_id = ?1 WHERE id = ?2 RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt.query_and_then([category_id.to_string(), id.to_string()], |row| {
            Transaction::try_from(row)
        })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    pub fn set_transaction_account(
        &self,
        id: Uuid,
        account_id: Uuid,
    ) -> crate::Result<Transaction> {
        let transaction = self.get_transaction(id)?;
        if transaction.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }
        let sql = match transaction.transaction_type() {
            TransactionType::Income => {
                "UPDATE transactions SET receiver_id = ?1, sender_id = NULL WHERE id = ?2 RETURNING *"
            }
            TransactionType::Expense => {
                "UPDATE transactions SET sender_id = ?1, receiver_id = NULL WHERE id = ?2 RETURNING *"
            }
            TransactionType::Transfer => {
                "UPDATE transactions SET sender_id = ?1 WHERE id = ?2 RETURNING *"
            }
        };
        let connection = self.connection();
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt.query_and_then([account_id.to_string(), id.to_string()], |row| {
            Transaction::try_from(row)
        })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    pub fn set_transaction_payee(&self, id: Uuid, account_id: Uuid) -> crate::Result<Transaction> {
        let transaction = self.get_transaction(id)?;
        if transaction.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }

        let sql = match transaction.transaction_type() {
            TransactionType::Income => {
                "UPDATE transactions SET sender_id = ?1, category_id = NULL WHERE id = ?2 RETURNING *"
            }
            TransactionType::Expense => {
                "UPDATE transactions SET receiver_id = ?1, category_id = NULL WHERE id = ?2 RETURNING *"
            }
            TransactionType::Transfer => {
                "UPDATE transactions SET receiver_id = ?1, category_id = NULL WHERE id = ?2 RETURNING *"
            }
        };
        let connection = self.connection();
        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt.query_and_then([account_id.to_string(), id.to_string()], |row| {
            Transaction::try_from(row)
        })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    pub fn set_transaction_outflow(&self, id: Uuid, amount: Money) -> crate::Result<Transaction> {
        let transaction = self.get_transaction(id)?;
        if transaction.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }

        let connection = self.connection();
        match transaction.transaction_type() {
            // Setting outflow on an income coverts it to an expense
            TransactionType::Income => {
                let sql = "UPDATE transactions SET amount = ?1, sender_id = ?2, receiver_id = null WHERE id = ?3 RETURNING *";
                let mut stmt = connection.prepare_cached(sql)?;
                let mut rows = stmt.query_and_then(
                    params![
                        amount.inner(),
                        transaction.receiver_id.unwrap().to_string(),
                        id.to_string()
                    ],
                    |row| Transaction::try_from(row),
                )?;
                let transaction = rows.next().unwrap()?;
                Ok(transaction)
            }
            _ => {
                let sql = "UPDATE transactions SET amount = ?1 WHERE id = ?2 RETURNING *";
                let mut stmt = connection.prepare_cached(sql)?;
                let mut rows = stmt
                    .query_and_then(params![amount.inner(), id.to_string()], |row| {
                        Transaction::try_from(row)
                    })?;
                let transaction = rows.next().unwrap()?;
                Ok(transaction)
            }
        }
    }

    pub fn set_transaction_inflow(&self, id: Uuid, amount: Money) -> crate::Result<Transaction> {
        let transaction = self.get_transaction(id)?;
        if transaction.audited {
            return Err(Error::new("Cannot edit audited transaction"));
        }

        let connection = self.connection();
        match transaction.transaction_type() {
            // Setting inflow on an expense converts it into an income.
            TransactionType::Expense => {
                let sql = "UPDATE transactions SET amount = ?1, receiver_id = ?2, sender_id = null WHERE id = ?3 RETURNING *";
                let mut stmt = connection.prepare_cached(sql)?;
                let mut rows = stmt.query_and_then(
                    params![
                        amount.inner(),
                        transaction.sender_id.unwrap().to_string(),
                        id.to_string()
                    ],
                    |row| Transaction::try_from(row),
                )?;
                let transaction = rows.next().unwrap()?;
                Ok(transaction)
            }
            TransactionType::Income => {
                let sql = "UPDATE transactions SET amount = ?1 WHERE id = ?2 RETURNING *";
                let mut stmt = connection.prepare_cached(sql)?;
                let mut rows = stmt
                    .query_and_then(params![amount.inner(), id.to_string()], |row| {
                        Transaction::try_from(row)
                    })?;
                let transaction = rows.next().unwrap()?;
                Ok(transaction)
            }
            TransactionType::Transfer => {
                let sql = "UPDATE transactions SET amount = ?1 WHERE id = ?2 RETURNING *";
                let mut stmt = connection.prepare_cached(sql)?;
                let mut rows = stmt
                    .query_and_then(params![amount.inner(), id.to_string()], |row| {
                        Transaction::try_from(row)
                    })?;
                let transaction = rows.next().unwrap()?;
                Ok(transaction)
            }
        }
    }

    /// Deletes the transaction.
    pub fn delete_transaction(&self, id: Uuid) -> crate::Result<()> {
        let connection = self.connection();
        let mut stmt = connection
            .prepare_cached("DELETE FROM transactions WHERE id = ? AND is_audited = FALSE")?;
        stmt.execute([id.to_string()])?;
        Ok(())
    }

    /// Deletes an account and all associated transactions.
    pub fn delete_account(&self, id: Uuid) -> crate::Result<()> {
        let connection = self.connection();

        let mut stmt =
            connection.prepare_cached("DELETE FROM transactions WHERE receiver_id = ?")?;
        stmt.execute([id.to_string()])?;
        let mut stmt = connection.prepare_cached("DELETE FROM transactions WHERE sender_id = ?")?;
        stmt.execute([id.to_string()])?;
        let mut stmt = connection.prepare_cached("DELETE FROM accounts WHERE id = ?")?;
        stmt.execute([id.to_string()])?;

        Ok(())
    }

    /// Duplicates a transaction, the new transaction will have all the same values as the old transaction
    /// except the `id`.
    pub fn duplicate_transaction(&self, id: Uuid) -> crate::Result<Transaction> {
        let connection = self.connection();
        let sql = "INSERT INTO transactions(id,sender_id,receiver_id,category_id,transaction_date,amount) \
        SELECT ?1,sender_id,receiver_id,category_id,transaction_date,amount FROM transactions \
        WHERE id = ?2 RETURNING *";

        let mut stmt = connection.prepare_cached(sql)?;
        let mut rows = stmt
            .query_and_then([Uuid::now_v7().to_string(), id.to_string()], |row| {
                Transaction::try_from(row)
            })?;
        let transaction = rows.next().unwrap()?;
        Ok(transaction)
    }

    /// Returns the total amount of money that is available to assign to budgets.
    ///
    /// Calculated by subtracting the total amount assigned to all budgets from the total
    /// income into cash accounts.
    pub fn left_to_assign(&self) -> crate::Result<Money> {
        let total_assignable = self.total_assignable()?;
        let total_assigned: Money = self
            .fetch_budgets()?
            .iter()
            .map(|budget| budget.amount)
            .sum();

        Ok(total_assignable - total_assigned)
    }

    /// Returns the total amount available to spend for a budget.
    pub fn available_to_spend(&self, category_id: Uuid, date: Date) -> crate::Result<Money> {
        let total_assigned = self.total_assigned(category_id, date)?;
        let total_spent = self.total_spent_in_category(category_id, date)?;
        let mut available = total_assigned - total_spent;
        let category = self.get_category(category_id)?;
        if let CategoryKind::CreditPayment { account_id } = category.kind {
            let transactions = self.fetch_transactions()?;
            let credit_expenses: Money = transactions
                .iter()
                .filter(|t| {
                    t.transaction_type() == TransactionType::Expense
                        && t.kind == TransactionKind::Regular
                })
                .filter(|t| t.sender_id == Some(account_id))
                .filter(|t| (t.date.year(), t.date.month()) <= (date.year(), date.month()))
                .map(|t| t.amount)
                .sum();
            available += credit_expenses;
        }

        Ok(available)
    }

    pub fn total_spent_in_category(&self, category_id: Uuid, date: Date) -> crate::Result<Money> {
        let category = self.get_category(category_id)?;
        let transactions = self.fetch_transactions()?;

        if let CategoryKind::CreditPayment { account_id } = category.kind {
            let total_spent: Money = transactions
                .iter()
                .filter(|t| t.transaction_type() == TransactionType::Transfer)
                .filter(|t| t.receiver_id == Some(account_id))
                .filter(|t| (t.date.year(), t.date.month()) <= (date.year(), date.month()))
                .map(|t| t.amount)
                .sum();
            return Ok(total_spent);
        }

        let total_spent: Money = transactions
            .iter()
            .filter(|t| t.transaction_type() == TransactionType::Expense)
            .filter(|t| t.category_id == Some(category_id))
            .filter(|t| (t.date.year(), t.date.month()) <= (date.year(), date.month()))
            .map(|t| t.amount)
            .sum();
        Ok(total_spent)
    }

    pub fn total_assigned(&self, category_id: Uuid, date: Date) -> crate::Result<Money> {
        let budgets = self.fetch_budgets()?;
        let total_assigned: Money = budgets
            .iter()
            .filter(|b| b.category_id == category_id)
            .filter(|b| (b.year, b.month) <= (date.year() as i64, date.month() as i64))
            .map(|b| b.amount)
            .sum();
        Ok(total_assigned)
    }

    pub fn available_to_spend_in_group(&self, id: Uuid, date: Date) -> crate::Result<Money> {
        let mut available = Money::ZERO;
        let groups = self.fetch_categories()?;
        let group = groups
            .iter()
            .find(|g| g.id == id)
            .ok_or(Error::new("Category group not found"))?;
        let categories = self.fetch_categories_in_group(group.id)?;
        for category in categories {
            available += self.available_to_spend(category.id, date)?;
        }
        Ok(available)
    }

    /// Returns the total amount of money deposited into cash accounts.
    fn total_assignable(&self) -> crate::Result<Money> {
        let cash_accounts: HashMap<Uuid, Account> = self
            .fetch_accounts()?
            .into_iter()
            .filter(|account| account.account_type == AccountType::Cash)
            .map(|account| (account.id, account))
            .collect();

        let mut total_cash = Money::ZERO;
        let transactions = self.fetch_transactions()?;

        for transaction in transactions {
            // Add money deposited into cash accounts
            if transaction.transaction_type() == TransactionType::Income
                && let Some(account_id) = transaction.receiver_id
                && cash_accounts.contains_key(&account_id)
            {
                total_cash += transaction.amount;
            }
        }

        Ok(total_cash)
    }

    /// Creates any missing credit account categories.
    pub fn check_credit_account_categories(&self) -> crate::Result<()> {
        let groups = self.fetch_categories()?;
        let credit_group = groups
            .iter()
            .find(|group| group.kind == GroupKind::CreditPayment)
            .ok_or(Error::new("Credit payments group does not exist"))?;

        let accounts = self.fetch_accounts()?;
        let iter = accounts
            .iter()
            .filter(|account| account.account_type == AccountType::Credit);

        let connection = self.connection();
        let sql =
            "INSERT INTO categories(id,title,group_id,account_id) VALUES(?1,?2,?3,?4) RETURNING *";
        let mut stmt = connection.prepare_cached(sql)?;

        for account in iter {
            let exists = credit_group
                .categories
                .iter()
                .find(|c| {
                    c.kind
                        == CategoryKind::CreditPayment {
                            account_id: account.id,
                        }
                })
                .is_some();
            if exists {
                continue;
            }

            let params = params![
                Uuid::now_v7().to_string(),
                account.name,
                credit_group.id.to_string(),
                account.id.to_string()
            ];
            stmt.execute(params)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use jiff::civil::date;
    use rusqlite::OptionalExtension;

    #[test]
    fn create_category_group() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let group = service.create_category_group("Wants")?;
        assert_eq!(group.title, "Wants");

        service.connection().query_one(
            "SELECT * FROM category_groups WHERE id = ?",
            [group.id.to_string()],
            |row| {
                let deleted_at: Option<i64> = row.get("deleted_at")?;
                let id: String = row.get("id")?;
                let title: String = row.get("title")?;
                let is_meta: bool = row.get("is_meta")?;

                assert!(deleted_at.is_none());
                assert_eq!(title, "Wants");
                assert!(!is_meta);
                assert_eq!(group.id.to_string(), id);
                Ok(())
            },
        )?;
        Ok(())
    }

    #[test]
    fn move_category() -> crate::Result<()> {
        let service = Service::open_in_memory()?;

        let group = service.create_category_group("")?;
        let group2 = service.create_category_group("")?;
        let category = service.create_category("Groceries", group.id)?;
        service.move_category(category.id, group2.id)?;

        let group = service.get_category_group(group.id)?;
        let group2 = service.get_category_group(group2.id)?;
        assert!(!group.categories.contains(&category));
        assert!(group2.categories.contains(&category));

        service
            .connection()
            .query_one("SELECT * FROM categories", [], |row| {
                let group_id: String = row.get("group_id")?;
                assert_eq!(group_id, group2.id.to_string());
                Ok(())
            })?;
        Ok(())
    }

    #[test]
    fn total_assignable() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let account = service.create_account("", AccountType::Cash)?;

        service
            .create_income()
            .amount(Money::new(25))
            .account(account.id)
            .submit()?;

        service
            .create_income()
            .amount(Money::new(75))
            .account(account.id)
            .submit()?;

        service
            .create_expense()
            .amount(Money::new(205))
            .account(account.id)
            .submit()?;

        let total_assignable = service.total_assignable()?;
        assert_eq!(total_assignable, Money::new(100));

        Ok(())
    }

    #[test]
    fn total_assignable_excludes_transfers() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let account = service.create_account("", AccountType::Cash)?;
        let account2 = service.create_account("", AccountType::Cash)?;

        service
            .create_income()
            .amount(Money::new(25))
            .account(account.id)
            .submit()?;

        service
            .create_income()
            .amount(Money::new(5))
            .account(account.id)
            .submit()?;

        service
            .create_transfer()
            .amount(Money::new(75))
            .accounts(account.id, account2.id)
            .submit()?;

        let total_assignable = service.total_assignable()?;
        assert_eq!(total_assignable, Money::new(30));

        Ok(())
    }

    #[test]
    fn create_missing_credit_categories() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let groups = service.fetch_categories()?;
        let credit_account = service.create_account("", AccountType::Credit)?;
        service.create_account("", AccountType::Cash)?;
        service.check_credit_account_categories()?;

        let groups = service.fetch_categories()?;
        let categories: Vec<_> = groups.into_iter().flat_map(|g| g.categories).collect();
        assert_eq!(categories.len(), 1);

        let category = &categories[0];
        assert_eq!(
            category.kind,
            CategoryKind::CreditPayment {
                account_id: credit_account.id
            }
        );
        Ok(())
    }

    #[test]
    fn create_missing_credit_categories_skips_existing_categories() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        service.create_account("", AccountType::Credit)?;
        service.create_account("", AccountType::Credit)?;
        service.check_credit_account_categories()?;

        let groups = service.fetch_categories()?;
        assert_eq!(groups[0].categories.len(), 2);

        service.check_credit_account_categories()?;
        let groups = service.fetch_categories()?;
        assert_eq!(groups[0].categories.len(), 2);
        Ok(())
    }

    #[test]
    fn create_expense() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let account = service.create_account("My account", AccountType::Cash)?;
        let group = service.create_category_group("")?;
        let category = service.create_category("Movies", group.id)?;
        let expense = service
            .create_expense()
            .amount(Money::from_f64(25.24))
            .date(date(20, 1, 1))
            .category(category.id)
            .account(account.id)
            .note("The odyssey")
            .submit()?;

        assert_eq!(expense.amount, Money::from_f64(25.24));
        assert_eq!(expense.sender_id.unwrap(), account.id);
        assert_eq!(expense.category_id.unwrap(), category.id);
        assert_eq!(expense.date, date(20, 1, 1));
        assert_eq!(expense.note.unwrap(), "The odyssey");
        assert!(expense.receiver_id.is_none());
        Ok(())
    }

    #[test]
    fn create_income() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let account = service.create_account("My account", AccountType::Cash)?;
        let income = service
            .create_income()
            .date(date(20, 1, 1))
            .account(account.id)
            .amount(Money::from_f64(25.24))
            .note("The odyssey")
            .submit()?;

        assert_eq!(income.amount, Money::from_f64(25.24));
        assert_eq!(income.receiver_id.unwrap(), account.id);
        assert!(income.category_id.is_none());
        assert_eq!(income.date, date(20, 1, 1));
        assert_eq!(income.note.unwrap(), "The odyssey");
        assert!(income.sender_id.is_none());
        Ok(())
    }

    #[test]
    fn create_transfer() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let account = service.create_account("My account", AccountType::Cash)?;
        let account2 = service.create_account("My account 2", AccountType::Cash)?;
        let transfer = service
            .create_transfer()
            .date(date(2100, 12, 1))
            .accounts(account.id, account2.id)
            .amount(Money::from_f64(25.24))
            .note("Transfer to savings")
            .submit()?;

        assert_eq!(transfer.amount, Money::from_f64(25.24));
        assert_eq!(transfer.sender_id.unwrap(), account.id);
        assert_eq!(transfer.receiver_id.unwrap(), account2.id);
        assert!(transfer.category_id.is_none());
        assert_eq!(transfer.date, date(2100, 12, 1));
        assert_eq!(transfer.note.unwrap(), "Transfer to savings");
        Ok(())
    }

    #[test]
    fn delete_category() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        let group = service.create_category_group("")?;
        let category = service.create_category("", group.id)?;
        service.delete_category(category.id)?;
        let row = service
            .connection()
            .query_row("SELECT * FROM categories", [], |row| {
                row.get::<_, String>("id")
            })
            .optional()?;
        assert!(row.is_none());
        Ok(())
    }

    #[test]
    fn delete_category_with_dependants() -> crate::Result<()> {
        let service = Service::open_in_memory()?;
        service.create_account("", AccountType::Cash)?;
        let group = service.create_category_group("")?;
        let category = service.create_category("", group.id)?;
        service.create_expense().category(category.id).submit()?;
        service.delete_category(category.id)?;
        let transactions = service.fetch_transactions()?;
        assert!(transactions[0].category_id.is_none());
        Ok(())
    }
}
