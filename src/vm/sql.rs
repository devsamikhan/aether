// ==============================================================================
// AetherSQL — Vector-Relational Hybrid SQL Engine
// High-Performance In-Memory SQL Querying & AI Vector Similarity over DataFrames
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::dataframe::{get_df, register_df, NativeDataFrame, TypedColumn};
use super::value::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// ==============================================================================
// 1. SQL AST & Parser
// ==============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum SqlExpr {
    Column(String),
    Literal(Value),
    VectorLiteral(Vec<f64>),
    BinaryOp {
        op: String,
        left: Box<SqlExpr>,
        right: Box<SqlExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<SqlExpr>,
    },
}

#[derive(Clone, Debug)]
pub struct SelectItem {
    pub expr: SqlExpr,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SqlQuery {
    pub select: Vec<SelectItem>,
    pub from: String,
    pub where_clause: Option<SqlExpr>,
    pub order_by: Option<(String, bool)>, // (column, descending)
    pub limit: Option<usize>,
}

pub struct SqlParser<'a> {
    tokens: Vec<&'a str>,
    pos: usize,
}

impl<'a> SqlParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut tokens = Vec::new();
        let mut chars = input.char_indices().peekable();
        let bytes = input.as_bytes();

        while let Some(&(i, ch)) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
                continue;
            }

            if ch == ',' || ch == '(' || ch == ')' || ch == '*' || ch == '+' || ch == '-' || ch == '/' {
                chars.next();
                tokens.push(&input[i..i + 1]);
                continue;
            }

            if ch == '\'' || ch == '"' {
                let quote = ch;
                let start = i;
                chars.next();
                let mut end = start + 1;
                while let Some(&(j, c)) = chars.peek() {
                    chars.next();
                    if c == quote {
                        end = j + c.len_utf8();
                        break;
                    }
                    end = j + c.len_utf8();
                }
                tokens.push(&input[start..end]);
                continue;
            }

            if ch == '[' {
                let start = i;
                chars.next();
                let mut end = start + 1;
                while let Some(&(j, c)) = chars.peek() {
                    chars.next();
                    if c == ']' {
                        end = j + 1;
                        break;
                    }
                    end = j + c.len_utf8();
                }
                tokens.push(&input[start..end]);
                continue;
            }

            if ch == '>' || ch == '<' || ch == '=' || ch == '!' {
                let start = i;
                chars.next();
                if let Some(&(_, next_c)) = chars.peek() {
                    if next_c == '=' || (ch == '<' && next_c == '>') {
                        chars.next();
                        tokens.push(std::str::from_utf8(&bytes[start..start + 2]).unwrap());
                        continue;
                    }
                }
                tokens.push(&input[start..start + 1]);
                continue;
            }

            // Word or number
            let start = i;
            let mut end = start;
            while let Some(&(j, c)) = chars.peek() {
                if c.is_whitespace()
                    || c == ','
                    || c == '('
                    || c == ')'
                    || c == '*'
                    || c == '+'
                    || c == '-'
                    || c == '/'
                    || c == '>'
                    || c == '<'
                    || c == '='
                    || c == '!'
                    || c == '['
                    || c == ']'
                {
                    break;
                }
                chars.next();
                end = j + c.len_utf8();
            }
            tokens.push(&input[start..end]);
        }

        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.pos).copied()
    }

    fn next_token(&mut self) -> Option<&'a str> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos];
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        if let Some(t) = self.peek() {
            if t.eq_ignore_ascii_case(kw) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    pub fn parse_query(&mut self) -> Result<SqlQuery, String> {
        if !self.match_keyword("SELECT") {
            return Err("Expected SELECT statement".into());
        }

        // Parse select items
        let mut select = Vec::new();
        loop {
            if self.peek() == Some("*") {
                self.next_token();
                select.push(SelectItem {
                    expr: SqlExpr::Column("*".into()),
                    alias: None,
                });
            } else {
                let expr = self.parse_expr()?;
                let mut alias = None;
                if self.match_keyword("AS") {
                    alias = self.next_token().map(|s| s.to_string());
                }
                select.push(SelectItem { expr, alias });
            }

            if self.peek() == Some(",") {
                self.next_token();
            } else {
                break;
            }
        }

        if !self.match_keyword("FROM") {
            return Err("Expected FROM clause".into());
        }

        let from = self.next_token().ok_or("Expected table name after FROM")?.to_string();

        // WHERE clause
        let mut where_clause = None;
        if self.match_keyword("WHERE") {
            where_clause = Some(self.parse_expr()?);
        }

        // ORDER BY clause
        let mut order_by = None;
        if self.match_keyword("ORDER") {
            if !self.match_keyword("BY") {
                return Err("Expected BY after ORDER".into());
            }
            let col = self.next_token().ok_or("Expected column after ORDER BY")?.to_string();
            let mut desc = false;
            if self.match_keyword("DESC") {
                desc = true;
            } else {
                let _ = self.match_keyword("ASC");
            }
            order_by = Some((col, desc));
        }

        // LIMIT clause
        let mut limit = None;
        if self.match_keyword("LIMIT") {
            let lim_str = self.next_token().ok_or("Expected number after LIMIT")?;
            limit = Some(lim_str.parse::<usize>().map_err(|_| "Invalid LIMIT number")?);
        }

        Ok(SqlQuery {
            select,
            from,
            where_clause,
            order_by,
            limit,
        })
    }

    fn parse_expr(&mut self) -> Result<SqlExpr, String> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<SqlExpr, String> {
        let mut left = self.parse_and_expr()?;
        while self.match_keyword("OR") {
            let right = self.parse_and_expr()?;
            left = SqlExpr::BinaryOp {
                op: "OR".into(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<SqlExpr, String> {
        let mut left = self.parse_comparison()?;
        while self.match_keyword("AND") {
            let right = self.parse_comparison()?;
            left = SqlExpr::BinaryOp {
                op: "AND".into(),
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<SqlExpr, String> {
        let left = self.parse_additive()?;
        if let Some(tok) = self.peek() {
            if tok == "=" || tok == "!=" || tok == "<>" || tok == ">" || tok == "<" || tok == ">=" || tok == "<=" {
                let op = self.next_token().unwrap().to_string();
                let right = self.parse_additive()?;
                return Ok(SqlExpr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<SqlExpr, String> {
        let mut left = self.parse_multiplicative()?;
        while let Some(tok) = self.peek() {
            if tok == "+" || tok == "-" {
                let op = self.next_token().unwrap().to_string();
                let right = self.parse_multiplicative()?;
                left = SqlExpr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<SqlExpr, String> {
        let mut left = self.parse_primary()?;
        while let Some(tok) = self.peek() {
            if tok == "*" || tok == "/" {
                let op = self.next_token().unwrap().to_string();
                let right = self.parse_primary()?;
                left = SqlExpr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<SqlExpr, String> {
        let tok = self.next_token().ok_or("Unexpected end of expression")?;

        // String literal: 'hello' or "world"
        if (tok.starts_with('\'') && tok.ends_with('\'')) || (tok.starts_with('"') && tok.ends_with('"')) {
            let unquoted = &tok[1..tok.len() - 1];
            return Ok(SqlExpr::Literal(Value::string(unquoted.to_string())));
        }

        // Number literal
        if let Ok(i) = tok.parse::<i64>() {
            return Ok(SqlExpr::Literal(Value::Int(i)));
        }
        if let Ok(f) = tok.parse::<f64>() {
            return Ok(SqlExpr::Literal(Value::Float(f)));
        }

        // Boolean literal
        if tok.eq_ignore_ascii_case("true") {
            return Ok(SqlExpr::Literal(Value::Bool(true)));
        }
        if tok.eq_ignore_ascii_case("false") {
            return Ok(SqlExpr::Literal(Value::Bool(false)));
        }

        // Vector literal: [1.0, 2.0, 3.0]
        if tok.starts_with('[') && tok.ends_with(']') {
            let inner = &tok[1..tok.len() - 1];
            let vec_vals: Vec<f64> = inner
                .split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            return Ok(SqlExpr::VectorLiteral(vec_vals));
        }

        // Function call: func(arg1, arg2)
        if self.peek() == Some("(") {
            self.next_token(); // '('
            let mut args = Vec::new();
            if self.peek() != Some(")") {
                loop {
                    args.push(self.parse_expr()?);
                    if self.peek() == Some(",") {
                        self.next_token();
                    } else {
                        break;
                    }
                }
            }
            if self.next_token() != Some(")") {
                return Err(format!("Expected closing ')' after function call '{}'", tok));
            }
            return Ok(SqlExpr::FunctionCall {
                name: tok.to_lowercase(),
                args,
            });
        }

        // Column identifier
        Ok(SqlExpr::Column(tok.to_string()))
    }
}

// ==============================================================================
// 2. Vector Similarity Mathematical Primitives
// ==============================================================================

pub fn vector_cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

pub fn vector_dot_product(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    let mut sum = 0.0;
    for i in 0..n {
        sum += a[i] * b[i];
    }
    sum
}

pub fn vector_l2_distance(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    let mut sum_sq = 0.0;
    for i in 0..n {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    sum_sq.sqrt()
}

// ==============================================================================
// 3. SQL Query Execution Engine
// ==============================================================================

pub fn execute_sql(query_str: &str, tables: &HashMap<String, Arc<Mutex<NativeDataFrame>>>) -> Result<NativeDataFrame, String> {
    let mut parser = SqlParser::new(query_str);
    let query = parser.parse_query()?;

    let df_arc = tables.get(&query.from)
        .ok_or_else(|| format!("Table '{}' not found in SQL context", query.from))?;
    let df = df_arc.lock().unwrap();

    let mut row_indices: Vec<usize> = (0..df.row_count).collect();

    // 1. WHERE Filtering
    if let Some(ref where_expr) = query.where_clause {
        let mut filtered_indices = Vec::new();
        for &r in &row_indices {
            let cond_val = eval_row_expr(where_expr, &df, r)?;
            if cond_val.is_truthy() {
                filtered_indices.push(r);
            }
        }
        row_indices = filtered_indices;
    }

    // 2. Compute Select Items / Derived Columns
    let mut result_columns = HashMap::new();
    let mut result_column_names = Vec::new();

    let is_wildcard = query.select.len() == 1 && matches!(query.select[0].expr, SqlExpr::Column(ref c) if c == "*");

    if is_wildcard {
        result_column_names = df.column_names.clone();
        for name in &df.column_names {
            let col = df.columns.get(name).unwrap();
            result_columns.insert(name.clone(), col.reorder_by_indices(&row_indices));
        }
    } else {
        for item in &query.select {
            let col_name = item.alias.clone().unwrap_or_else(|| match &item.expr {
                SqlExpr::Column(c) => c.clone(),
                SqlExpr::FunctionCall { name, .. } => name.clone(),
                _ => "computed".to_string(),
            });

            let mut values = Vec::with_capacity(row_indices.len());
            for &r in &row_indices {
                let v = eval_row_expr(&item.expr, &df, r)?;
                values.push(v);
            }

            result_columns.insert(col_name.clone(), TypedColumn::from_values(&values));
            result_column_names.push(col_name);
        }
    }

    let mut out_df = NativeDataFrame::new(result_column_names, result_columns);

    // 3. ORDER BY
    if let Some((ref order_col, descending)) = query.order_by {
        out_df = out_df.sort_by(order_col, descending)?;
    }

    // 4. LIMIT
    if let Some(lim) = query.limit {
        out_df = out_df.head(lim);
    }

    Ok(out_df)
}

fn eval_row_expr(expr: &SqlExpr, df: &NativeDataFrame, row_idx: usize) -> Result<Value, String> {
    match expr {
        SqlExpr::Literal(v) => Ok(v.clone()),

        SqlExpr::VectorLiteral(v) => {
            let vals = v.iter().map(|&f| Value::Float(f)).collect();
            Ok(Value::array(vals))
        }

        SqlExpr::Column(name) => {
            let col = df.columns.get(name)
                .ok_or_else(|| format!("Column '{}' not found in table", name))?;
            col.get(row_idx).ok_or_else(|| format!("Row {} out of bounds for column '{}'", row_idx, name))
        }

        SqlExpr::BinaryOp { op, left, right } => {
            let a = eval_row_expr(left, df, row_idx)?;
            let b = eval_row_expr(right, df, row_idx)?;

            match op.as_str() {
                "=" => Ok(Value::Bool(a == b)),
                "!=" | "<>" => Ok(Value::Bool(a != b)),
                ">" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x > y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x > y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Bool(x as f64 > y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(x > y as f64)),
                    (a, b) => Err(format!("Cannot compare '>' on types {} and {}", a.type_name(), b.type_name())),
                },
                "<" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x < y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x < y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((x as f64) < y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(x < y as f64)),
                    (a, b) => Err(format!("Cannot compare '<' on types {} and {}", a.type_name(), b.type_name())),
                },
                ">=" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x >= y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x >= y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Bool(x as f64 >= y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(x >= y as f64)),
                    (a, b) => Err(format!("Cannot compare '>=' on types {} and {}", a.type_name(), b.type_name())),
                },
                "<=" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x <= y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x <= y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((x as f64) <= y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(x <= y as f64)),
                    (a, b) => Err(format!("Cannot compare '<=' on types {} and {}", a.type_name(), b.type_name())),
                },
                "AND" => Ok(Value::Bool(a.is_truthy() && b.is_truthy())),
                "OR" => Ok(Value::Bool(a.is_truthy() || b.is_truthy())),
                "+" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 + y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + y as f64)),
                    _ => Err("Invalid operands for '+'".into()),
                },
                "-" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x - y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 - y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x - y as f64)),
                    _ => Err("Invalid operands for '-'".into()),
                },
                "*" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x * y)),
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 * y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x * y as f64)),
                    _ => Err("Invalid operands for '*'".into()),
                },
                "/" => match (a, b) {
                    (Value::Int(x), Value::Int(y)) => {
                        if y == 0 { Err("Division by zero".into()) } else { Ok(Value::Float(x as f64 / y as f64)) }
                    }
                    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
                    (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 / y)),
                    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x / y as f64)),
                    _ => Err("Invalid operands for '/'".into()),
                },
                unknown => Err(format!("Unsupported operator '{}'", unknown)),
            }
        }

        SqlExpr::FunctionCall { name, args } => match name.as_str() {
            "vector_cosine" | "cosine_similarity" => {
                if args.len() < 2 {
                    return Err("vector_cosine(col, query_vec) requires 2 arguments".into());
                }
                let a = extract_f64_vec(&eval_row_expr(&args[0], df, row_idx)?)?;
                let b = extract_f64_vec(&eval_row_expr(&args[1], df, row_idx)?)?;
                Ok(Value::Float(vector_cosine_similarity(&a, &b)))
            }

            "vector_dot" | "dot_product" => {
                if args.len() < 2 {
                    return Err("vector_dot(col, query_vec) requires 2 arguments".into());
                }
                let a = extract_f64_vec(&eval_row_expr(&args[0], df, row_idx)?)?;
                let b = extract_f64_vec(&eval_row_expr(&args[1], df, row_idx)?)?;
                Ok(Value::Float(vector_dot_product(&a, &b)))
            }

            "vector_l2" | "euclidean_dist" => {
                if args.len() < 2 {
                    return Err("vector_l2(col, query_vec) requires 2 arguments".into());
                }
                let a = extract_f64_vec(&eval_row_expr(&args[0], df, row_idx)?)?;
                let b = extract_f64_vec(&eval_row_expr(&args[1], df, row_idx)?)?;
                Ok(Value::Float(vector_l2_distance(&a, &b)))
            }

            "round" => {
                let v = eval_row_expr(&args[0], df, row_idx)?;
                match v {
                    Value::Float(f) => Ok(Value::Float(f.round())),
                    Value::Int(i) => Ok(Value::Int(i)),
                    _ => Ok(v),
                }
            }

            "abs" => {
                let v = eval_row_expr(&args[0], df, row_idx)?;
                match v {
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    Value::Int(i) => Ok(Value::Int(i.abs())),
                    _ => Ok(v),
                }
            }

            unknown => Err(format!("Unknown SQL function '{}'", unknown)),
        },
    }
}

fn extract_f64_vec(val: &Value) -> Result<Vec<f64>, String> {
    match val {
        Value::Array(arr) => Ok(arr
            .lock()
            .iter()
            .map(|v| match v {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => 0.0,
            })
            .collect()),
        _ => Err("Expected array/vector".into()),
    }
}

// ==============================================================================
// 4. Global SQL Table Registry & VM Bindings
// ==============================================================================

static SQL_TABLES: OnceLock<Mutex<HashMap<String, Arc<Mutex<NativeDataFrame>>>>> = OnceLock::new();

fn get_sql_tables() -> &'static Mutex<HashMap<String, Arc<Mutex<NativeDataFrame>>>> {
    SQL_TABLES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_sql_table(name: &str, df: Arc<Mutex<NativeDataFrame>>) {
    get_sql_tables().lock().unwrap().insert(name.to_lowercase(), df);
}

pub fn register_sql_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Sql.register_table(name, df_id)
    mod_map.insert(
        "register_table".to_string(),
        Value::Native("Sql.register_table".into(), |args| {
            if args.len() < 2 {
                return Err("Sql.register_table(name, df_id) requires 2 arguments".into());
            }
            let name = args[0].to_string();
            let df_id = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };

            let df = get_df(df_id)?;
            register_sql_table(&name, df);
            Ok(Value::Bool(true))
        }),
    );

    // 2. Sql.query(sql_string, extra_tables_map?) -> result_df_id
    mod_map.insert(
        "query".to_string(),
        Value::Native("Sql.query".into(), |args| {
            if args.is_empty() {
                return Err("Sql.query(sql_string, extra_tables_map?) requires sql string".into());
            }
            let sql_str = args[0].to_string();

            let mut tables = get_sql_tables().lock().unwrap().clone();

            if args.len() > 1 {
                if let Value::Map(m) = &args[1] {
                    for (t_name, t_val) in m.lock().iter() {
                        let df_id = match t_val {
                            Value::Int(i) => *i as u64,
                            Value::ClassInstance(inst) => {
                                if let Some(Value::Int(i)) = inst.fields.lock().get("id") {
                                    *i as u64
                                } else {
                                    0
                                }
                            }
                            _ => 0,
                        };
                        if df_id > 0 {
                            if let Ok(df_arc) = get_df(df_id) {
                                tables.insert(t_name.to_lowercase(), df_arc);
                            }
                        }
                    }
                }
            }

            let result_df = execute_sql(&sql_str, &tables)?;
            let id = register_df(result_df);
            Ok(Value::Int(id as i64))
        }),
    );

    globals.insert("Sql".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_sql".to_string(), Value::map(mod_map));
}
