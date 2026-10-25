use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "report_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub report_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub section: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
