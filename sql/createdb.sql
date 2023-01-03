-- Create commands to set up mysql DBs/tables. This was manually run via `mysql` cli.
CREATE DATABASE compute_broker;
CREATE TABLE hosts (
     id BINARY(16) PRIMARY KEY,
     hostname VARCHAR(256) UNIQUE NOT NULL,
     info VARCHAR(256) NOT NULL,
     alloc_state TINYINT NOT NULL,
     health_state TINYINT NOT NULL
);
