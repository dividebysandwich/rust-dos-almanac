use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Games {
    Table,
    Id,
    Name,
    Slug,
    Aliases,
    SearchText,
    Year,
    Publisher,
    Hidden,
    SourceHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GameVersions {
    Table,
    Id,
    GameId,
    Label,
    Hidden,
    SourceHash,
    CreatedAt,
}

#[derive(DeriveIden)]
enum VersionFiles {
    Table,
    Id,
    VersionId,
    Filename,
    Size,
    Crc32,
    Md5,
    Sha1,
}

#[derive(DeriveIden)]
enum VersionArchives {
    Table,
    Id,
    VersionId,
    RaMd5,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
    Role,
    Disabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Reports {
    Table,
    Id,
    GameId,
    VersionId,
    Rating,
    Description,
    RustDosVersion,
    UserId,
    SourceHash,
    Hidden,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ReportSettings {
    Table,
    ReportId,
    Section,
    Key,
    Value,
}

#[derive(DeriveIden)]
enum Blessings {
    Table,
    Id,
    GameId,
    /// The version id, or 0 for the whole game: NULLs never collide in a
    /// unique index, so the game-wide blessing needs a real value.
    VersionKey,
    ReportId,
    UserId,
    BlessedAt,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    TokenHash,
    UserId,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum ModerationLog {
    Table,
    Id,
    UserId,
    Action,
    TargetType,
    TargetId,
    Detail,
    At,
}

#[derive(DeriveIden)]
enum ServerSecrets {
    Table,
    Name,
    Value,
}

fn fk(from_table: impl IntoIden, from_col: impl IntoIden, to_table: impl IntoIden, to_col: impl IntoIden) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .from(from_table, from_col)
        .to(to_table, to_col)
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Games::Table)
                    .col(pk_auto(Games::Id))
                    .col(string(Games::Name))
                    .col(string_uniq(Games::Slug))
                    .col(text(Games::Aliases).default(""))
                    .col(text(Games::SearchText))
                    .col(integer_null(Games::Year))
                    .col(string_null(Games::Publisher))
                    .col(boolean(Games::Hidden).default(false))
                    .col(string_null(Games::SourceHash))
                    .col(big_integer(Games::CreatedAt))
                    .col(big_integer(Games::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GameVersions::Table)
                    .col(pk_auto(GameVersions::Id))
                    .col(integer(GameVersions::GameId))
                    .col(string(GameVersions::Label))
                    .col(boolean(GameVersions::Hidden).default(false))
                    .col(string_null(GameVersions::SourceHash))
                    .col(big_integer(GameVersions::CreatedAt))
                    .foreign_key(&mut fk(GameVersions::Table, GameVersions::GameId, Games::Table, Games::Id))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(Index::create().name("idx_versions_game").table(GameVersions::Table).col(GameVersions::GameId).to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(VersionFiles::Table)
                    .col(pk_auto(VersionFiles::Id))
                    .col(integer(VersionFiles::VersionId))
                    .col(string(VersionFiles::Filename))
                    .col(big_integer_null(VersionFiles::Size))
                    .col(string_len_null(VersionFiles::Crc32, 8))
                    .col(string_len_null(VersionFiles::Md5, 32))
                    .col(string_len_null(VersionFiles::Sha1, 40))
                    .foreign_key(&mut fk(VersionFiles::Table, VersionFiles::VersionId, GameVersions::Table, GameVersions::Id))
                    .to_owned(),
            )
            .await?;
        for (name, col) in [
            ("idx_files_version", VersionFiles::VersionId),
            ("idx_files_name", VersionFiles::Filename),
            ("idx_files_crc32", VersionFiles::Crc32),
            ("idx_files_md5", VersionFiles::Md5),
            ("idx_files_sha1", VersionFiles::Sha1),
        ] {
            manager.create_index(Index::create().name(name).table(VersionFiles::Table).col(col).to_owned()).await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(VersionArchives::Table)
                    .col(pk_auto(VersionArchives::Id))
                    .col(integer(VersionArchives::VersionId))
                    .col(string_len_uniq(VersionArchives::RaMd5, 32))
                    .foreign_key(&mut fk(VersionArchives::Table, VersionArchives::VersionId, GameVersions::Table, GameVersions::Id))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .col(pk_auto(Users::Id))
                    .col(string_uniq(Users::Username))
                    .col(string(Users::PasswordHash))
                    .col(string(Users::Role))
                    .col(boolean(Users::Disabled).default(false))
                    .col(big_integer(Users::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Reports::Table)
                    .col(pk_auto(Reports::Id))
                    .col(integer(Reports::GameId))
                    .col(integer_null(Reports::VersionId))
                    .col(string(Reports::Rating))
                    .col(text_null(Reports::Description))
                    .col(string_null(Reports::RustDosVersion))
                    .col(integer_null(Reports::UserId))
                    .col(string(Reports::SourceHash))
                    .col(boolean(Reports::Hidden).default(false))
                    .col(big_integer(Reports::CreatedAt))
                    .col(big_integer(Reports::UpdatedAt))
                    .foreign_key(&mut fk(Reports::Table, Reports::GameId, Games::Table, Games::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Reports::Table, Reports::VersionId)
                            .to(GameVersions::Table, GameVersions::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Reports::Table, Reports::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, col) in [
            ("idx_reports_game", Reports::GameId),
            ("idx_reports_source", Reports::SourceHash),
            ("idx_reports_created", Reports::CreatedAt),
        ] {
            manager.create_index(Index::create().name(name).table(Reports::Table).col(col).to_owned()).await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(ReportSettings::Table)
                    .col(integer(ReportSettings::ReportId))
                    .col(string(ReportSettings::Section))
                    .col(string(ReportSettings::Key))
                    .col(string(ReportSettings::Value))
                    .primary_key(Index::create().col(ReportSettings::ReportId).col(ReportSettings::Section).col(ReportSettings::Key))
                    .foreign_key(&mut fk(ReportSettings::Table, ReportSettings::ReportId, Reports::Table, Reports::Id))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Blessings::Table)
                    .col(pk_auto(Blessings::Id))
                    .col(integer(Blessings::GameId))
                    .col(integer(Blessings::VersionKey))
                    .col(integer(Blessings::ReportId))
                    .col(integer_null(Blessings::UserId))
                    .col(big_integer(Blessings::BlessedAt))
                    .foreign_key(&mut fk(Blessings::Table, Blessings::GameId, Games::Table, Games::Id))
                    .foreign_key(&mut fk(Blessings::Table, Blessings::ReportId, Reports::Table, Reports::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Blessings::Table, Blessings::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_blessings_target")
                    .table(Blessings::Table)
                    .col(Blessings::GameId)
                    .col(Blessings::VersionKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .col(string_len(Sessions::TokenHash, 64).primary_key())
                    .col(integer(Sessions::UserId))
                    .col(big_integer(Sessions::CreatedAt))
                    .col(big_integer(Sessions::ExpiresAt))
                    .foreign_key(&mut fk(Sessions::Table, Sessions::UserId, Users::Table, Users::Id))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ModerationLog::Table)
                    .col(pk_auto(ModerationLog::Id))
                    .col(integer_null(ModerationLog::UserId))
                    .col(string(ModerationLog::Action))
                    .col(string(ModerationLog::TargetType))
                    .col(big_integer(ModerationLog::TargetId))
                    .col(text_null(ModerationLog::Detail))
                    .col(big_integer(ModerationLog::At))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ModerationLog::Table, ModerationLog::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerSecrets::Table)
                    .col(string_len(ServerSecrets::Name, 64).primary_key())
                    .col(string(ServerSecrets::Value))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            ServerSecrets::Table.into_iden(),
            ModerationLog::Table.into_iden(),
            Sessions::Table.into_iden(),
            Blessings::Table.into_iden(),
            ReportSettings::Table.into_iden(),
            Reports::Table.into_iden(),
            Users::Table.into_iden(),
            VersionArchives::Table.into_iden(),
            VersionFiles::Table.into_iden(),
            GameVersions::Table.into_iden(),
            Games::Table.into_iden(),
        ] {
            manager.drop_table(Table::drop().table(table).to_owned()).await?;
        }
        Ok(())
    }
}
