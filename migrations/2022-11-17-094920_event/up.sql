CREATE TABLE EVENT(
    ID VARCHAR PRIMARY KEY NOT NULL,
    TS BIGINT NOT NULL,
    SENDER VARCHAR NOT NULL,
    EVENT_TYPE VARCHAR NOT NULL
);
