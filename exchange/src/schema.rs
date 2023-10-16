// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "currency_status"))]
    pub struct CurrencyStatus;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "order_status"))]
    pub struct OrderStatus;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "order_type"))]
    pub struct OrderType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tx_status"))]
    pub struct TxStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::CurrencyStatus;

    currencies (id) {
        id -> Int4,
        #[max_length = 9]
        symbol -> Bpchar,
        name -> Text,
        status -> CurrencyStatus,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TxStatus;

    deposits (id) {
        id -> Uuid,
        user_id -> Uuid,
        currency_id -> Int4,
        amount -> Int8,
        status -> TxStatus,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::OrderType;
    use super::sql_types::OrderStatus;

    orders (id) {
        id -> Uuid,
        user_id -> Uuid,
        currency_id -> Int4,
        #[sql_name = "type"]
        type_ -> OrderType,
        amount -> Int8,
        filled_amount -> Int8,
        remaining_amount -> Nullable<Int8>,
        price -> Int8,
        status -> OrderStatus,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    transactions (id) {
        id -> Uuid,
        buyer_order_id -> Uuid,
        seller_order_id -> Uuid,
        amount -> Int8,
        price -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        password_hash -> Bytea,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    wallets (id) {
        id -> Uuid,
        user_id -> Uuid,
        currency_id -> Int4,
        balance -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TxStatus;

    withdrawals (id) {
        id -> Uuid,
        user_id -> Uuid,
        currency_id -> Int4,
        amount -> Int8,
        status -> TxStatus,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(deposits -> currencies (currency_id));
diesel::joinable!(deposits -> users (user_id));
diesel::joinable!(orders -> currencies (currency_id));
diesel::joinable!(orders -> users (user_id));
diesel::joinable!(wallets -> currencies (currency_id));
diesel::joinable!(wallets -> users (user_id));
diesel::joinable!(withdrawals -> currencies (currency_id));
diesel::joinable!(withdrawals -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    currencies,
    deposits,
    orders,
    transactions,
    users,
    wallets,
    withdrawals,
);
