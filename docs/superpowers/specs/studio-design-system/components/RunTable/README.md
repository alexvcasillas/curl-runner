# RunTable

Results of a collection run, one row per request, 36px rows with hairline separators.

- Columns: request (method + name), status pill, time with a proportional bar, checks passed/total, attempts.
- Numbers use tabular figures; empty cells show an em dash.
- Failed rows sort to the top after a run; the selected row opens in the inspector.
- GPUI-kit: `Table` with a custom delegate.
