use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "version_archives")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub version_id: i32,
    #[sea_orm(unique)]
    pub ra_md5: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
