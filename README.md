# Postgreth: Ethereum on Postgres.

Postgreth is a Postgresql extension enabling ethabi datatypes inside of SQL.

## Status

- Currently not for public usage (no extensive testing, CI/CD, nor a release process).
- Used internally to parse 300GB worth of events.
- Will be production ready soon (tm).

## Usage

After installing the extension, run:

```sql
CREATE extension postgreth
```

Now parsing of ethabi becomes available:

```sql
/*
returns a JSONB object:

{
    // the name of the event (e.g. "Transfer")
    "name": ...,
    // The fields of the event, keyed.
    "data": {...}
}
*/
select log_to_jsonb(
    /* abi.json, only the events key is required. */
    abi::text,
    /* log JSON as found in receipt.logs */
    log::text, 
)
```