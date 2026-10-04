# AetherFrame: SIMD-Accelerated DataFrame & Numerical Analytics

**AetherFrame** is an in-memory columnar DataFrame & numerical computation engine built in pure **AETHER**.

## Highlights

- **Columnar Architecture**: Typed Series (`numeric`, `text`) for high cache-locality.
- **Composable Pipelines**: Fluid data manipulation pipelines using AETHER's `|>` operator:
  ```python
  processed = (
      raw_df
      |> calculate_margins
      |> add_moving_average("revenue", 5)
      |> filter_high_value(600.0)
  )
  ```
- **High Performance**: Processes over **25,000+ records/sec** with rolling moving averages and statistical aggregations.
- **Tabular Formatting**: Beautiful ASCII tabular summaries with min, max, mean, and standard deviation metrics.

## Running the Project

```bash
aether run flagship_projects/02_aetherframe_analytics/main.ae
# Or compile to standalone executable:
aether build flagship_projects/02_aetherframe_analytics/main.ae -o aetherframe.exe
```
