CREATE TABLE state (
    channel text NOT NULL,
    payload text NOT NULL,

    PRIMARY KEY(channel)
);

-- INSERT INTO state (channel, payload) VALUES ('13', '{"reward":"Lorem ipsum"}')
