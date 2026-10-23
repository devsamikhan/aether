// ==============================================================================
// AETHER Native Regular Expression Engine
// Supports ^, $, ., *, +, ?, {n,m}, \d, \D, \w, \W, \s, \S, [...], [^...], |
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use std::collections::HashMap;
use super::value::Value;

#[derive(Debug, Clone)]
enum RegexAtom {
    Any,
    Literal(char),
    Digit(bool),      // true = \d, false = \D
    Word(bool),       // true = \w, false = \W
    Space(bool),      // true = \s, false = \S
    CharClass {
        negated: bool,
        singles: Vec<char>,
        ranges: Vec<(char, char)>,
    },
    Group(Box<RegexPattern>),
}

#[derive(Debug, Clone)]
struct QuantifiedAtom {
    atom: RegexAtom,
    min: usize,
    max: Option<usize>, // None = unbounded
}

#[derive(Debug, Clone)]
struct RegexBranch {
    start_anchor: bool,
    end_anchor: bool,
    seq: Vec<QuantifiedAtom>,
}

#[derive(Debug, Clone)]
pub struct RegexPattern {
    branches: Vec<RegexBranch>,
}

impl RegexPattern {
    pub fn compile(pattern: &str) -> Result<Self, String> {
        let alts = split_top_level_alternations(pattern);
        let mut branches = Vec::with_capacity(alts.len());
        for alt in alts {
            branches.push(parse_branch(alt)?);
        }
        Ok(Self { branches })
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.find_span(text).is_some()
    }

    pub fn find_span(&self, text: &str) -> Option<(usize, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let mut byte_offsets: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        byte_offsets.push(text.len());

        for branch in &self.branches {
            if branch.start_anchor {
                if let Some(end_idx) = match_seq(&branch.seq, &chars, 0, branch.end_anchor) {
                    return Some((byte_offsets[0], byte_offsets[end_idx]));
                }
            } else {
                for start_idx in 0..=chars.len() {
                    if let Some(end_idx) =
                        match_seq(&branch.seq, &chars, start_idx, branch.end_anchor)
                    {
                        return Some((byte_offsets[start_idx], byte_offsets[end_idx]));
                    }
                }
            }
        }
        None
    }

    pub fn find(&self, text: &str) -> Option<String> {
        self.find_span(text).map(|(s, e)| text[s..e].to_string())
    }

    pub fn find_all(&self, text: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut offset = 0;
        while offset <= text.len() {
            let sub = &text[offset..];
            if let Some((s, e)) = self.find_span(sub) {
                if s == e {
                    if offset < text.len() {
                        let ch_len = sub.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                        offset += s + ch_len;
                    } else {
                        break;
                    }
                } else {
                    results.push(sub[s..e].to_string());
                    offset += e;
                }
            } else {
                break;
            }
        }
        results
    }

    pub fn replace(&self, text: &str, replacement: &str) -> String {
        if let Some((s, e)) = self.find_span(text) {
            let mut out = String::with_capacity(text.len() + replacement.len());
            out.push_str(&text[..s]);
            out.push_str(replacement);
            out.push_str(&text[e..]);
            out
        } else {
            text.to_string()
        }
    }

    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut offset = 0;
        while offset <= text.len() {
            let sub = &text[offset..];
            if let Some((s, e)) = self.find_span(sub) {
                if s == e {
                    if let Some(ch) = sub.chars().next() {
                        out.push(ch);
                        offset += ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    out.push_str(&sub[..s]);
                    out.push_str(replacement);
                    offset += e;
                }
            } else {
                out.push_str(sub);
                break;
            }
        }
        out
    }

    pub fn split(&self, text: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut offset = 0;
        while offset <= text.len() {
            let sub = &text[offset..];
            if let Some((s, e)) = self.find_span(sub) {
                if s == e {
                    break;
                }
                parts.push(sub[..s].to_string());
                offset += e;
            } else {
                parts.push(sub.to_string());
                break;
            }
        }
        parts
    }
}

fn split_top_level_alternations(pattern: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_brackets = false;
    let mut escaped = false;
    let mut start = 0usize;

    for (idx, ch) in pattern.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '[' && !in_brackets {
            in_brackets = true;
            continue;
        }
        if ch == ']' && in_brackets {
            in_brackets = false;
            continue;
        }
        if !in_brackets {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' && depth > 0 {
                depth -= 1;
            } else if ch == '|' && depth == 0 {
                out.push(&pattern[start..idx]);
                start = idx + 1;
            }
        }
    }
    out.push(&pattern[start..]);
    out
}

fn parse_branch(mut pat: &str) -> Result<RegexBranch, String> {
    let start_anchor = if pat.starts_with('^') {
        pat = &pat[1..];
        true
    } else {
        false
    };
    let end_anchor = if pat.ends_with('$') && !pat.ends_with("\\$") {
        pat = &pat[..pat.len() - 1];
        true
    } else {
        false
    };

    let chars: Vec<char> = pat.chars().collect();
    let mut i = 0;
    let mut seq = Vec::new();

    while i < chars.len() {
        let atom = match chars[i] {
            '.' => {
                i += 1;
                RegexAtom::Any
            }
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    return Err("RegexError: Trailing backslash".to_string());
                }
                let esc = chars[i];
                i += 1;
                match esc {
                    'd' => RegexAtom::Digit(true),
                    'D' => RegexAtom::Digit(false),
                    'w' => RegexAtom::Word(true),
                    'W' => RegexAtom::Word(false),
                    's' => RegexAtom::Space(true),
                    'S' => RegexAtom::Space(false),
                    'n' => RegexAtom::Literal('\n'),
                    'r' => RegexAtom::Literal('\r'),
                    't' => RegexAtom::Literal('\t'),
                    other => RegexAtom::Literal(other),
                }
            }
            '[' => {
                i += 1;
                let mut negated = false;
                if i < chars.len() && chars[i] == '^' {
                    negated = true;
                    i += 1;
                }
                let mut singles = Vec::new();
                let mut ranges = Vec::new();
                while i < chars.len() && chars[i] != ']' {
                    let c = if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        chars[i]
                    } else {
                        chars[i]
                    };
                    if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
                        let end_c = chars[i + 2];
                        ranges.push((c, end_c));
                        i += 3;
                    } else {
                        singles.push(c);
                        i += 1;
                    }
                }
                if i >= chars.len() || chars[i] != ']' {
                    return Err("RegexError: Unclosed character class '['".to_string());
                }
                i += 1; // consume ']'
                RegexAtom::CharClass {
                    negated,
                    singles,
                    ranges,
                }
            }
            '(' => {
                i += 1;
                let start = i;
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if chars[i] == '(' {
                        depth += 1;
                    } else if chars[i] == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                if depth != 0 {
                    return Err("RegexError: Unclosed group '('".to_string());
                }
                let inner_str: String = chars[start..i].iter().collect();
                i += 1; // consume ')'
                RegexAtom::Group(Box::new(RegexPattern::compile(&inner_str)?))
            }
            c => {
                i += 1;
                RegexAtom::Literal(c)
            }
        };

        // Parse optional quantifier: *, +, ?, {n}, {n,m}
        let (min, max) = if i < chars.len() {
            match chars[i] {
                '*' => {
                    i += 1;
                    (0, None)
                }
                '+' => {
                    i += 1;
                    (1, None)
                }
                '?' => {
                    i += 1;
                    (0, Some(1))
                }
                '{' => {
                    i += 1;
                    let q_start = i;
                    while i < chars.len() && chars[i] != '}' {
                        i += 1;
                    }
                    if i >= chars.len() {
                        return Err("RegexError: Unclosed quantifier '{'".to_string());
                    }
                    let q_str: String = chars[q_start..i].iter().collect();
                    i += 1; // consume '}'
                    if let Some((a, b)) = q_str.split_once(',') {
                        let min_v = a.trim().parse::<usize>().unwrap_or(0);
                        let max_v = if b.trim().is_empty() {
                            None
                        } else {
                            Some(b.trim().parse::<usize>().unwrap_or(min_v))
                        };
                        (min_v, max_v)
                    } else {
                        let exact = q_str.trim().parse::<usize>().unwrap_or(1);
                        (exact, Some(exact))
                    }
                }
                _ => (1, Some(1)),
            }
        } else {
            (1, Some(1))
        };

        seq.push(QuantifiedAtom { atom, min, max });
    }

    Ok(RegexBranch {
        start_anchor,
        end_anchor,
        seq,
    })
}

fn match_seq(
    seq: &[QuantifiedAtom],
    chars: &[char],
    pos: usize,
    end_anchor: bool,
) -> Option<usize> {
    if seq.is_empty() {
        if !end_anchor || pos == chars.len() {
            return Some(pos);
        } else {
            return None;
        }
    }

    let first = &seq[0];
    let rest = &seq[1..];

    // Collect all reachable end positions for `count` repetitions of `first.atom`
    let mut positions = vec![pos];
    let mut step_positions = Vec::new();
    step_positions.push((0usize, pos));

    let max_reps = first.max.unwrap_or(chars.len().saturating_sub(pos) + 1);
    let mut count = 0usize;

    while count < max_reps {
        let mut next_positions = Vec::new();
        for &p in &positions {
            if let Some(np) = match_atom(&first.atom, chars, p) {
                if np > p || count == 0 {
                    if !next_positions.contains(&np) {
                        next_positions.push(np);
                    }
                }
            }
        }
        if next_positions.is_empty() {
            break;
        }
        count += 1;
        for &np in &next_positions {
            step_positions.push((count, np));
        }
        positions = next_positions;
    }

    // Greedy backtracking: try longest repetition counts first
    for &(reps, next_pos) in step_positions.iter().rev() {
        if reps >= first.min {
            if let Some(final_pos) = match_seq(rest, chars, next_pos, end_anchor) {
                return Some(final_pos);
            }
        }
    }

    None
}

fn match_atom(atom: &RegexAtom, chars: &[char], pos: usize) -> Option<usize> {
    match atom {
        RegexAtom::Group(sub_pat) => {
            for branch in &sub_pat.branches {
                if let Some(end_p) = match_seq(&branch.seq, chars, pos, false) {
                    return Some(end_p);
                }
            }
            None
        }
        _ => {
            if pos >= chars.len() {
                return None;
            }
            let c = chars[pos];
            let matched = match atom {
                RegexAtom::Any => c != '\n',
                RegexAtom::Literal(lit) => c == *lit,
                RegexAtom::Digit(pos_match) => c.is_ascii_digit() == *pos_match,
                RegexAtom::Word(pos_match) => (c.is_alphanumeric() || c == '_') == *pos_match,
                RegexAtom::Space(pos_match) => c.is_whitespace() == *pos_match,
                RegexAtom::CharClass {
                    negated,
                    singles,
                    ranges,
                } => {
                    let in_class = singles.contains(&c)
                        || ranges.iter().any(|(lo, hi)| c >= *lo && c <= *hi);
                    if *negated { !in_class } else { in_class }
                }
                RegexAtom::Group(_) => unreachable!(),
            };
            if matched { Some(pos + 1) } else { None }
        }
    }
}

pub fn register_regex_module(globals: &mut HashMap<String, Value>) {
    let mut regex_mod = HashMap::new();

    regex_mod.insert(
        "is_match".to_string(),
        Value::Native("Regex.is_match".into(), |args| {
            if args.len() < 2 {
                return Err("Regex.is_match(pattern, text) requires 2 arguments".into());
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            Ok(Value::Bool(pat.is_match(&text)))
        }),
    );

    regex_mod.insert(
        "find".to_string(),
        Value::Native("Regex.find".into(), |args| {
            if args.len() < 2 {
                return Err("Regex.find(pattern, text) requires 2 arguments".into());
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            match pat.find(&text) {
                Some(m) => Ok(Value::string(m)),
                None => Ok(Value::Nil),
            }
        }),
    );

    regex_mod.insert(
        "find_all".to_string(),
        Value::Native("Regex.find_all".into(), |args| {
            if args.len() < 2 {
                return Err("Regex.find_all(pattern, text) requires 2 arguments".into());
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            let matches: Vec<Value> = pat.find_all(&text).into_iter().map(Value::string).collect();
            Ok(Value::array(matches))
        }),
    );

    regex_mod.insert(
        "replace".to_string(),
        Value::Native("Regex.replace".into(), |args| {
            if args.len() < 3 {
                return Err("Regex.replace(pattern, text, replacement) requires 3 arguments".into());
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            let rep = format!("{}", args[2]);
            Ok(Value::string(pat.replace(&text, &rep)))
        }),
    );

    regex_mod.insert(
        "replace_all".to_string(),
        Value::Native("Regex.replace_all".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "Regex.replace_all(pattern, text, replacement) requires 3 arguments".into(),
                );
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            let rep = format!("{}", args[2]);
            Ok(Value::string(pat.replace_all(&text, &rep)))
        }),
    );

    regex_mod.insert(
        "split".to_string(),
        Value::Native("Regex.split".into(), |args| {
            if args.len() < 2 {
                return Err("Regex.split(pattern, text) requires 2 arguments".into());
            }
            let pat = RegexPattern::compile(&format!("{}", args[0]))?;
            let text = format!("{}", args[1]);
            let parts: Vec<Value> = pat.split(&text).into_iter().map(Value::string).collect();
            Ok(Value::array(parts))
        }),
    );

    globals.insert("Regex".to_string(), Value::map(regex_mod));
}
