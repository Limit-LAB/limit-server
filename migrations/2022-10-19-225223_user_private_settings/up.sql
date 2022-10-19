CREATE TABLE USER_PRIVACY_SETTINGS(
    ID VARCHAR PRIMARY KEY NOT NULL,
    -- VISIBILITY
    AVATAR VARCHAR NOT NULL,
    -- VISIBILITY
    LAST_SEEN VARCHAR NOT NULL,
    -- VISIBILITY
    JOINED_GROUPS VARCHAR NOT NULL,
    -- VISIBILITY
    FORWARDS VARCHAR NOT NULL,
    -- DURATION
    JWT_EXPIRATION VARCHAR NOT NULL,

    FOREIGN KEY(ID) REFERENCES USER(ID)
);
