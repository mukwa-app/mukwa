// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Wakunguma Kalimukwa

use uuid::Uuid;

use crate::Money;

pub enum StoreError {
    NotFound { message: String },
}

pub struct Account {}
pub struct Category {}
pub struct CategoryGroup {
    categories: Vec<Category>,
}

pub struct Budget {}
pub struct Transaction {
    confirmed: bool,
}

pub struct UpdateTransaction<'s, S: Store> {
    id: Uuid,
    store: &'s mut S,
}

impl<'s, S: Store> UpdateTransaction<'s, S> {
    fn note(mut self, note: &str) -> Self {
        self
    }

    fn submit(self) -> Result<(), StoreError> {
        Ok(())
    }
}

pub trait Store: Clone {
    // TODO: methods might need to be &mut
    fn fetch_accounts(&self) -> Result<Vec<Account>, StoreError>;
    fn fetch_categories(&self) -> Result<Vec<CategoryGroup>, StoreError>;
    fn fetch_budgets(&self) -> Result<Vec<Budget>, StoreError>;
    fn fetch_transactions(&self) -> Result<Vec<Transaction>, StoreError>;

    fn delete_category(&mut self, id: Uuid) -> Result<(), StoreError>;
    fn delete_category_group(&mut self, id: Uuid) -> Result<(), StoreError>;
    fn delete_transaction(&mut self, id: Uuid) -> Result<(), StoreError>;
    fn delete_budget_target(&mut self, category_id: Uuid) -> Result<(), StoreError>;

    /// Creates or updates the budget target for a category.
    fn set_budget_target(&mut self, category_id: Uuid, amount: Money) -> Result<(), StoreError>;
    /// Sets the `is_audited` field of all the account's transactions to `true`.
    fn audit_account(&mut self, id: Uuid) -> Result<(), StoreError>;
    fn unlock_transactions(&mut self, ids: &[Uuid]) -> Result<(), StoreError>;
    fn confirm_transactions(&mut self, ids: &[Uuid]) -> Result<(), StoreError>;
    fn move_category(&mut self, id: Uuid, group_id: Uuid) -> Result<(), StoreError>;
    fn edit_transaction<'s>(
        &'s mut self,
        id: Uuid,
    ) -> Result<UpdateTransaction<'s, Self>, StoreError>;
}
