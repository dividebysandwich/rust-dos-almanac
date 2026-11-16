use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "game_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub game_id: i32,
    pub label: String,
    pub hidden: bool,
    pub source_hash: Option<String>,
    /// `dos`, `win16` or `win32`; None when unknown.
    pub platform: Option<String>,
    /// SHA-256 of the key of whoever added it, who may correct it; cleared
    /// once staff edit it.
    pub owner_key: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
