// @generated automatically by Diesel CLI.

diesel::table! {
    MESSAGE (ID) {
        ID -> Text,
        TS -> BigInt,
        SENDER -> Text,
        RECEIVER_ID -> Text,
        RECEIVER_SERVER -> Text,
        TEXT -> Text,
        EXTENSIONS -> Text,
    }
}

diesel::table! {
    USER (ID) {
        ID -> Text,
        PUBKEY -> Text,
        SHAREDKEY -> Text,
    }
}

diesel::table! {
    USER_LOGIN_PASSCODE (ID) {
        ID -> Text,
        PASSCODE -> Text,
    }
}

diesel::table! {
    USER_PRIVACY_SETTINGS (ID) {
        ID -> Text,
        AVATAR -> Text,
        LAST_SEEN -> Text,
        JOINED_GROUPS -> Text,
        FORWARDS -> Text,
        JWT_EXPIRATION -> Text,
    }
}

diesel::table! {
    USER_PROFILE (ID) {
        ID -> Text,
        NAME -> Text,
        USER_NAME -> Text,
        BIO -> Nullable<Text>,
        AVATAR -> Nullable<Text>,
        LAST_SEEN -> Nullable<Text>,
        LAST_MODIFIED -> Nullable<Text>,
    }
}

diesel::joinable!(USER_LOGIN_PASSCODE -> USER (ID));
diesel::joinable!(USER_PRIVACY_SETTINGS -> USER (ID));
diesel::joinable!(USER_PROFILE -> USER (ID));

diesel::allow_tables_to_appear_in_same_query!(
    MESSAGE,
    USER,
    USER_LOGIN_PASSCODE,
    USER_PRIVACY_SETTINGS,
    USER_PROFILE,
);
