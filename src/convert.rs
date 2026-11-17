//! COBOL(固定/自由形式)の部分集合 → 共通IR → Python / Rust。
//! 対応: PROGRAM-ID / 01〜49・77(PIC, VALUE) / DISPLAY / MOVE / ADD / SUBTRACT / MULTIPLY / DIVIDE / COMPUTE /
//!       IF-ELSE-END-IF(AND・OR・NOT・比較) / PERFORM(段落・n TIMES・UNTIL・VARYING・インライン) / 段落 / STOP RUN・GOBACK。
//! 未対応の文(ファイル I/O・EVALUATE・GO TO・CALL 等)は TODO コメントとして出力し、黙って落とさない。
//! 数値は整数(i64)として扱う(小数 PIC は整数に丸めた近似で、TODO を付ける)。
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Str(String),
    Var(String),
    Bin(char, Box<Expr>, Box<Expr>), // + - * / ^
    Neg(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    Cmp(String, Expr, Expr), // "==" "!=" ">" "<" ">=" "<="
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    Not(Box<Cond>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Loop {
    Once,
    Times(Expr),
    Until(Cond),
    Varying { var: String, from: Expr, by: Expr, until: Cond },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Display(Vec<Expr>),
    Move(Expr, Vec<String>),
    Assign(String, Expr),      // x = e
    AddTo(String, Expr),       // x += e
    SubFrom(String, Expr),     // x -= e
    MulBy(String, Expr),       // x *= e
    If(Cond, Vec<Stmt>, Vec<Stmt>),
    Perform(Loop, Vec<Stmt>),
    Call(String),              // PERFORM 段落
    Stop,
    Nop,
    Todo(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub name: String,
    pub numeric: bool,
    pub init: String,
    pub decimal: bool,
    pub undeclared: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct Program {
    pub id: String,
    pub vars: Vec<Var>,
    pub stmts: Vec<Stmt>,
    pub paragraphs: Vec<(String, Vec<Stmt>)>,
}

fn strip(src: &str, free: bool) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let l: String = if free {
                l.to_string()
            } else {
                let c: Vec<char> = l.chars().collect();
                if c.len() <= 6 || c[6] == '*' || c[6] == '/' {
                    return None;
                }
                c[6..c.len().min(72)].iter().collect()
            };
            if l.trim_start().starts_with("*>") || l.trim().is_empty() {
                None
            } else {
                Some(l)
            }
        })
        .collect()
}

fn ident(n: &str) -> String {
    n.to_lowercase().replace('-', "_")
}

#[derive(Debug, Clone)]
struct Tok {
    s: String,
    dot: bool,
}

/// 引用符内の空白を保持し、括弧を独立したトークンにし、文末の '.' は dot フラグにする。
fn tokenize(s: &str) -> Vec<Tok> {
    let mut raw: Vec<String> = vec![];
    let mut cur = String::new();
    let mut q: Option<char> = None;
    for c in s.chars() {
        match q {
            Some(qc) => {
                cur.push(c);
                if c == qc {
                    q = None;
                }
            }
            None if c == '"' || c == '\'' => {
                cur.push(c);
                q = Some(c);
            }
            None if c.is_whitespace() || c == ',' || c == ';' => {
                if !cur.is_empty() {
                    raw.push(std::mem::take(&mut cur));
                }
            }
            None if c == '(' || c == ')' => {
                if !cur.is_empty() {
                    raw.push(std::mem::take(&mut cur));
                }
                raw.push(c.to_string());
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        raw.push(cur);
    }
    let mut v: Vec<Tok> = vec![];
    for r in raw {
        if r == "." {
            if let Some(l) = v.last_mut() {
                l.dot = true;
            }
            continue;
        }
        if r.ends_with('.') && r.len() > 1 {
            v.push(Tok { s: r[..r.len() - 1].to_string(), dot: true });
        } else {
            v.push(Tok { s: r, dot: false });
        }
    }
    v
}

const STARTERS: [&str; 34] = [
    "DISPLAY", "MOVE", "ADD", "SUBTRACT", "MULTIPLY", "DIVIDE", "COMPUTE", "IF", "ELSE", "END-IF", "PERFORM", "END-PERFORM", "STOP", "GOBACK", "EXIT", "CONTINUE", "OPEN", "CLOSE", "READ", "WRITE", "CALL", "EVALUATE", "END-EVALUATE", "WHEN", "ACCEPT", "GO", "INITIALIZE", "SET", "STRING", "UNSTRING", "INSPECT", "SORT", "EXEC", "REWRITE",
];

fn is_starter(t: &str) -> bool {
    STARTERS.contains(&t.to_uppercase().as_str())
}

struct P {
    t: Vec<Tok>,
    i: usize,
    ended: bool, // 直前に消費したトークンが文末(.)だった
    used: BTreeSet<String>,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn up(&self, k: usize) -> String {
        self.t.get(self.i + k).map(|t| t.s.to_uppercase()).unwrap_or_default()
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.t.get(self.i).cloned();
        if let Some(x) = &t {
            self.ended = x.dot;
            self.i += 1;
        }
        t
    }
    fn word(&self) -> String {
        self.up(0)
    }

    // ---- 式 ----
    fn atom(&mut self) -> Result<Expr, String> {
        let t = self.next().ok_or("式が途中で終わった")?;
        let s = t.s;
        if s == "(" {
            let e = self.expr(0)?;
            if self.peek().map(|x| x.s == ")").unwrap_or(false) {
                self.next();
            }
            return Ok(e);
        }
        if s == "-" {
            return Ok(Expr::Neg(Box::new(self.atom()?)));
        }
        let c = s.chars().next().unwrap_or(' ');
        if c == '"' || c == '\'' {
            return Ok(Expr::Str(s[1..s.len().saturating_sub(1)].to_string()));
        }
        if c.is_ascii_digit() || (c == '-' && s.len() > 1) {
            return Ok(Expr::Num(s));
        }
        match s.to_uppercase().as_str() {
            "ZERO" | "ZEROS" | "ZEROES" => return Ok(Expr::Num("0".into())),
            "SPACE" | "SPACES" => return Ok(Expr::Str(String::new())),
            _ => {}
        }
        if s.to_uppercase() == "FUNCTION" {
            return Err("FUNCTION は未対応".into());
        }
        let n = ident(&s);
        self.used.insert(n.clone());
        Ok(Expr::Var(n))
    }
    fn prec(op: &str) -> Option<(u8, char)> {
        match op {
            "+" => Some((1, '+')),
            "-" => Some((1, '-')),
            "*" => Some((2, '*')),
            "/" => Some((2, '/')),
            "**" => Some((3, '^')),
            _ => None,
        }
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let mut l = self.atom()?;
        while let Some(t) = self.peek() {
            let Some((p, op)) = Self::prec(&t.s) else { break };
            if p < min || p == 0 {
                break;
            }
            self.next();
            let r = self.expr(p + 1)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    /// 演算子を含まない1つ分(ADD/SUBTRACT の項、DISPLAY の項)
    fn operand(&mut self) -> Result<Expr, String> {
        self.atom()
    }

    // ---- 条件 ----
    fn cond(&mut self) -> Result<Cond, String> {
        let mut l = self.cond_and()?;
        while self.word() == "OR" {
            self.next();
            let r = self.cond_and()?;
            l = Cond::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn cond_and(&mut self) -> Result<Cond, String> {
        let mut l = self.cond_not()?;
        while self.word() == "AND" {
            self.next();
            let r = self.cond_not()?;
            l = Cond::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn cond_not(&mut self) -> Result<Cond, String> {
        if self.word() == "NOT" {
            self.next();
            return Ok(Cond::Not(Box::new(self.cond_not()?)));
        }
        if self.word() == "(" {
            let save = self.i;
            self.next();
            if let Ok(c) = self.cond() {
                if self.peek().map(|x| x.s == ")").unwrap_or(false) {
                    self.next();
                    return Ok(c);
                }
            }
            self.i = save; // 式の括弧だった
        }
        let a = self.expr(1)?;
        let mut neg = false;
        let mut w = self.word();
        if w == "IS" {
            self.next();
            w = self.word();
        }
        if w == "NOT" {
            neg = true;
            self.next();
            w = self.word();
        }
        let op = match w.as_str() {
            "=" | "EQUAL" => {
                self.next();
                if self.word() == "TO" {
                    self.next();
                }
                "=="
            }
            ">" | "GREATER" => {
                self.next();
                if self.word() == "THAN" {
                    self.next();
                }
                if self.word() == "OR" && self.up(1) == "EQUAL" {
                    self.next();
                    self.next();
                    if self.word() == "TO" {
                        self.next();
                    }
                    ">="
                } else {
                    ">"
                }
            }
            "<" | "LESS" => {
                self.next();
                if self.word() == "THAN" {
                    self.next();
                }
                if self.word() == "OR" && self.up(1) == "EQUAL" {
                    self.next();
                    self.next();
                    if self.word() == "TO" {
                        self.next();
                    }
                    "<="
                } else {
                    "<"
                }
            }
            ">=" => {
                self.next();
                ">="
            }
            "<=" => {
                self.next();
                "<="
            }
            other => return Err(format!("条件の形を解釈できない: {other}")),
        };
        let b = self.expr(1)?;
        let c = Cond::Cmp(op.to_string(), a, b);
        Ok(if neg { Cond::Not(Box::new(c)) } else { c })
    }

    // ---- 文 ----
    fn skip_to_end(&mut self, until: &[&str]) {
        // 未対応の文: 文末(.)、次の動詞、または指定語まで読み飛ばす
        let first = self.next();
        if first.map(|t| t.dot).unwrap_or(true) {
            return;
        }
        while let Some(t) = self.peek() {
            if is_starter(&t.s) || until.contains(&t.s.to_uppercase().as_str()) {
                break;
            }
            let d = t.dot;
            self.next();
            if d {
                break;
            }
        }
    }

    fn names_until(&mut self, stops: &[&str]) -> Vec<String> {
        let mut v = vec![];
        while let Some(t) = self.peek() {
            let u = t.s.to_uppercase();
            if is_starter(&u) || stops.contains(&u.as_str()) || t.s == ")" {
                break;
            }
            let (s, d) = (t.s.clone(), t.dot);
            self.next();
            if u != "ROUNDED" {
                let n = ident(&s);
                self.used.insert(n.clone());
                v.push(n);
            }
            if d {
                break;
            }
        }
        v
    }

    fn operands_until(&mut self, stops: &[&str]) -> Result<Vec<Expr>, String> {
        let mut v = vec![];
        loop {
            let Some(t) = self.peek() else { break };
            let u = t.s.to_uppercase();
            if is_starter(&u) || stops.contains(&u.as_str()) || self.ended && !v.is_empty() {
                break;
            }
            if self.ended && v.is_empty() {
                // 直前で文が終わっている(空の文)
                break;
            }
            v.push(self.operand()?);
            if self.ended {
                break;
            }
        }
        Ok(v)
    }

    fn sum(es: Vec<Expr>) -> Expr {
        let mut it = es.into_iter();
        let mut acc = it.next().unwrap_or(Expr::Num("0".into()));
        for e in it {
            acc = Expr::Bin('+', Box::new(acc), Box::new(e));
        }
        acc
    }

    fn block(&mut self, scoped: bool, terms: &[&str]) -> Vec<Stmt> {
        let mut v = vec![];
        loop {
            if scoped && self.ended {
                break;
            }
            let Some(t) = self.peek() else { break };
            let u = t.s.to_uppercase();
            if terms.contains(&u.as_str()) {
                break;
            }
            if !scoped {
                self.ended = false;
            }
            match self.statement() {
                Some(s) => v.push(s),
                None => break,
            }
        }
        v
    }

    fn statement(&mut self) -> Option<Stmt> {
        let t = self.peek()?.clone();
        let u = t.s.to_uppercase();
        let st = match u.as_str() {
            "DISPLAY" => {
                self.next();
                let ops = self.operands_until(&["UPON", "WITH", "NO"]);
                match ops {
                    Ok(o) => Stmt::Display(o),
                    Err(e) => {
                        self.skip_to_end(&[]);
                        Stmt::Todo(format!("DISPLAY: {e}"))
                    }
                }
            }
            "MOVE" => {
                self.next();
                match self.operand() {
                    Ok(src) if self.word() == "TO" => {
                        self.next();
                        let tg = self.names_until(&[]);
                        Stmt::Move(src, tg)
                    }
                    _ => {
                        self.skip_to_end(&[]);
                        Stmt::Todo("MOVE".into())
                    }
                }
            }
            "ADD" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let ops = self.operands_until(&["TO", "GIVING"])?;
                    let mut to = vec![];
                    let mut giving = vec![];
                    if self.word() == "TO" {
                        self.next();
                        to = self.operands_until(&["GIVING"])?;
                    }
                    if self.word() == "GIVING" {
                        self.next();
                        giving = self.names_until(&[]);
                    }
                    if giving.is_empty() {
                        let total = Self::sum(ops);
                        let mut out = vec![];
                        for t in to {
                            if let Expr::Var(n) = t {
                                out.push(Stmt::AddTo(n, total.clone()));
                            }
                        }
                        return Ok(if out.len() == 1 { out.remove(0) } else { Stmt::If(Cond::Cmp("==".into(), Expr::Num("1".into()), Expr::Num("1".into())), out, vec![]) });
                    }
                    let mut all = ops;
                    all.extend(to);
                    let total = Self::sum(all);
                    let mut out: Vec<Stmt> = giving.into_iter().map(|g| Stmt::Assign(g, total.clone())).collect();
                    Ok(if out.len() == 1 { out.remove(0) } else { Stmt::If(Cond::Cmp("==".into(), Expr::Num("1".into()), Expr::Num("1".into())), out, vec![]) })
                })();
                r.unwrap_or_else(|e| Stmt::Todo(format!("ADD: {e}")))
            }
            "SUBTRACT" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let ops = self.operands_until(&["FROM"])?;
                    if self.word() != "FROM" {
                        return Err("FROM が無い".into());
                    }
                    self.next();
                    let from = self.operand()?;
                    let sum = Self::sum(ops);
                    if self.word() == "GIVING" {
                        self.next();
                        let g = self.names_until(&[]);
                        return Ok(Stmt::Assign(g.into_iter().next().ok_or("GIVING 先が無い")?, Expr::Bin('-', Box::new(from), Box::new(sum))));
                    }
                    match from {
                        Expr::Var(n) => Ok(Stmt::SubFrom(n, sum)),
                        _ => Err("FROM の先が変数ではない".into()),
                    }
                })();
                r.unwrap_or_else(|e| Stmt::Todo(format!("SUBTRACT: {e}")))
            }
            "MULTIPLY" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let a = self.operand()?;
                    if self.word() != "BY" {
                        return Err("BY が無い".into());
                    }
                    self.next();
                    let b = self.operand()?;
                    if self.word() == "GIVING" {
                        self.next();
                        let g = self.names_until(&[]);
                        return Ok(Stmt::Assign(g.into_iter().next().ok_or("GIVING 先が無い")?, Expr::Bin('*', Box::new(a), Box::new(b))));
                    }
                    match b {
                        Expr::Var(n) => Ok(Stmt::MulBy(n, a)),
                        _ => Err("BY の先が変数ではない".into()),
                    }
                })();
                r.unwrap_or_else(|e| Stmt::Todo(format!("MULTIPLY: {e}")))
            }
            "DIVIDE" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let a = self.operand()?;
                    let by = self.word() == "BY";
                    if !(by || self.word() == "INTO") {
                        return Err("INTO/BY が無い".into());
                    }
                    self.next();
                    let b = self.operand()?;
                    if self.word() == "GIVING" {
                        self.next();
                        let g = self.names_until(&["REMAINDER"]);
                        if self.word() == "REMAINDER" {
                            return Err("REMAINDER は未対応".into());
                        }
                        let e = if by { Expr::Bin('/', Box::new(a), Box::new(b)) } else { Expr::Bin('/', Box::new(b), Box::new(a)) };
                        return Ok(Stmt::Assign(g.into_iter().next().ok_or("GIVING 先が無い")?, e));
                    }
                    match b {
                        Expr::Var(n) if !by => Ok(Stmt::Assign(n.clone(), Expr::Bin('/', Box::new(Expr::Var(n)), Box::new(a)))),
                        _ => Err("DIVIDE の形が未対応".into()),
                    }
                })();
                r.unwrap_or_else(|e| {
                    self.skip_to_end(&[]);
                    Stmt::Todo(format!("DIVIDE: {e}"))
                })
            }
            "COMPUTE" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let tg = self.names_until(&["="]);
                    if self.word() != "=" {
                        return Err("= が無い".into());
                    }
                    self.next();
                    let e = self.expr(1)?;
                    Ok(Stmt::Assign(tg.into_iter().next().ok_or("代入先が無い")?, e))
                })();
                r.unwrap_or_else(|e| {
                    self.skip_to_end(&[]);
                    Stmt::Todo(format!("COMPUTE: {e}"))
                })
            }
            "IF" => {
                self.next();
                match self.cond() {
                    Ok(c) => {
                        if self.word() == "THEN" {
                            self.next();
                        }
                        let th = self.block(true, &["ELSE", "END-IF"]);
                        let mut el = vec![];
                        if self.word() == "ELSE" {
                            self.next();
                            el = self.block(true, &["END-IF"]);
                        }
                        if self.word() == "END-IF" {
                            let d = self.peek().map(|t| t.dot).unwrap_or(false);
                            self.next();
                            self.ended = d;
                        }
                        Stmt::If(c, th, el)
                    }
                    Err(e) => {
                        // END-IF か文末まで読み飛ばす
                        while let Some(t) = self.peek() {
                            let end_if = t.s.to_uppercase() == "END-IF";
                            let d = t.dot;
                            self.next();
                            if end_if || d {
                                break;
                            }
                        }
                        Stmt::Todo(format!("IF: {e}"))
                    }
                }
            }
            "EVALUATE" => {
                self.next();
                let r: Result<Stmt, String> = (|| {
                    let is_true = self.word() == "TRUE";
                    let subj = if is_true {
                        self.next();
                        None
                    } else {
                        Some(self.expr(1)?)
                    };
                    let mut arms: Vec<(Option<Cond>, Vec<Stmt>)> = vec![];
                    while self.word() == "WHEN" {
                        let mut conds: Vec<Cond> = vec![];
                        let mut other = false;
                        while self.word() == "WHEN" {
                            self.next();
                            if self.word() == "OTHER" {
                                self.next();
                                other = true;
                                continue;
                            }
                            let c = if let Some(s) = &subj {
                                let a = self.expr(1)?;
                                if self.word() == "THRU" || self.word() == "THROUGH" {
                                    self.next();
                                    let b = self.expr(1)?;
                                    Cond::And(
                                        Box::new(Cond::Cmp(">=".into(), s.clone(), a)),
                                        Box::new(Cond::Cmp("<=".into(), s.clone(), b)),
                                    )
                                } else {
                                    Cond::Cmp("==".into(), s.clone(), a)
                                }
                            } else {
                                self.cond()?
                            };
                            if self.word() == "ALSO" {
                                return Err("ALSO(複数主語)は未対応".into());
                            }
                            conds.push(c);
                        }
                        let body = self.block(true, &["WHEN", "END-EVALUATE"]);
                        let c = if other {
                            None
                        } else {
                            let mut it = conds.into_iter();
                            let mut c = it.next().ok_or("WHEN の条件が無い")?;
                            for n in it {
                                c = Cond::Or(Box::new(c), Box::new(n));
                            }
                            Some(c)
                        };
                        arms.push((c, body));
                    }
                    if self.word() == "END-EVALUATE" {
                        let d = self.peek().map(|t| t.dot).unwrap_or(false);
                        self.next();
                        self.ended = d;
                    }
                    let mut tail: Vec<Stmt> = vec![];
                    for (c, body) in arms.into_iter().rev() {
                        tail = match c {
                            None => body,
                            Some(c) => vec![Stmt::If(c, body, tail)],
                        };
                    }
                    Ok(if tail.len() == 1 {
                        tail.remove(0)
                    } else {
                        Stmt::If(Cond::Cmp("==".into(), Expr::Num("1".into()), Expr::Num("1".into())), tail, vec![])
                    })
                })();
                r.unwrap_or_else(|e| {
                    while let Some(t) = self.peek() {
                        let end = t.s.to_uppercase() == "END-EVALUATE";
                        let d = t.dot;
                        self.next();
                        if end || d {
                            break;
                        }
                    }
                    Stmt::Todo(format!("EVALUATE: {e}"))
                })
            }
            "PERFORM" => {
                self.next();
                let w = self.word();
                let inline = w == "VARYING" || w == "UNTIL" || (self.up(1) == "TIMES");
                let mut call: Option<String> = None;
                if !inline && w != "VARYING" && w != "UNTIL" {
                    let n = self.next().unwrap();
                    call = Some(ident(&n.s));
                }
                let r: Result<Loop, String> = (|| {
                    let w = self.word();
                    if w == "VARYING" {
                        self.next();
                        let v = ident(&self.next().ok_or("VARYING 変数")?.s);
                        self.used.insert(v.clone());
                        if self.word() != "FROM" {
                            return Err("FROM が無い".into());
                        }
                        self.next();
                        let from = self.expr(1)?;
                        if self.word() != "BY" {
                            return Err("BY が無い".into());
                        }
                        self.next();
                        let by = self.expr(1)?;
                        if self.word() != "UNTIL" {
                            return Err("UNTIL が無い".into());
                        }
                        self.next();
                        let until = self.cond()?;
                        Ok(Loop::Varying { var: v, from, by, until })
                    } else if w == "UNTIL" {
                        self.next();
                        Ok(Loop::Until(self.cond()?))
                    } else if self.up(1) == "TIMES" {
                        let n = self.operand()?;
                        self.next();
                        Ok(Loop::Times(n))
                    } else if self.up(1) == "TIMES" || (call.is_some() && matches!(self.up(1).as_str(), "TIMES")) {
                        Ok(Loop::Once)
                    } else if call.is_some() && self.up(0).chars().all(|c| c.is_ascii_digit()) && self.up(1) == "TIMES" {
                        Ok(Loop::Once)
                    } else {
                        Ok(Loop::Once)
                    }
                })();
                match r {
                    Err(e) => {
                        self.skip_to_end(&["END-PERFORM"]);
                        Stmt::Todo(format!("PERFORM: {e}"))
                    }
                    Ok(mut lp) => {
                        // 段落 n TIMES の形(段落名の後ろに数と TIMES)
                        if let (Some(_), Loop::Once) = (&call, &lp) {
                            if self.up(1) == "TIMES" {
                                if let Ok(n) = self.operand() {
                                    self.next();
                                    lp = Loop::Times(n);
                                }
                            } else if self.word() == "UNTIL" {
                                self.next();
                                if let Ok(c) = self.cond() {
                                    lp = Loop::Until(c);
                                }
                            }
                        }
                        let body = match call {
                            Some(c) => vec![Stmt::Call(c)],
                            None => {
                                let b = self.block(false, &["END-PERFORM"]);
                                if self.word() == "END-PERFORM" {
                                    let d = self.peek().map(|t| t.dot).unwrap_or(false);
                                    self.next();
                                    self.ended = d;
                                }
                                b
                            }
                        };
                        Stmt::Perform(lp, body)
                    }
                }
            }
            "STOP" => {
                self.next();
                if self.word() == "RUN" {
                    self.next();
                }
                Stmt::Stop
            }
            "GOBACK" => {
                self.next();
                Stmt::Stop
            }
            "EXIT" | "CONTINUE" => {
                self.next();
                if self.word() == "PROGRAM" {
                    self.next();
                    Stmt::Stop
                } else {
                    Stmt::Nop
                }
            }
            "ELSE" | "END-IF" | "END-PERFORM" | "WHEN" | "END-EVALUATE" => {
                // 対応する開始が無い終端語: 読み飛ばす
                self.next();
                Stmt::Nop
            }
            other if is_starter(other) => {
                self.skip_to_end(&[]);
                Stmt::Todo(other.to_string())
            }
            _ => {
                // 段落名(文頭の語+ピリオド)は呼び出し側で処理済み。ここに来るのは未知の語
                self.next();
                Stmt::Todo(format!("未知の語 {}", t.s))
            }
        };
        Some(st)
    }
}

fn level(t: &str) -> bool {
    t.len() == 2 && t.chars().all(|c| c.is_ascii_digit()) && (t == "77" || t.parse::<u8>().map(|n| (1..=49).contains(&n)).unwrap_or(false))
}

pub fn parse(src: &str, free: bool) -> Program {
    let text = strip(src, free).join(" ");
    let toks = tokenize(&text);
    let mut p = Program::default();
    let proc_at = (0..toks.len()).find(|&i| toks[i].s.to_uppercase() == "PROCEDURE" && toks.get(i + 1).map(|t| t.s.to_uppercase() == "DIVISION").unwrap_or(false));
    let head_end = proc_at.unwrap_or(toks.len());
    // データ部
    let mut i = 0;
    while i < head_end {
        let u = toks[i].s.to_uppercase();
        if u == "PROGRAM-ID" {
            p.id = toks.get(i + 1).map(|t| t.s.clone()).unwrap_or_default();
            i += 2;
        } else if level(&toks[i].s) && !toks[i].dot {
            let name = toks.get(i + 1).map(|t| t.s.clone()).unwrap_or_default();
            let mut j = i + 2;
            while j < head_end && !toks[j - 1].dot && !(j == i + 2 && toks[i + 1].dot) {
                if toks[j].dot {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let seg: Vec<String> = toks[(i + 2).min(head_end)..j.min(head_end)].iter().map(|t| t.s.clone()).collect();
            let mut numeric = false;
            let mut decimal = false;
            let mut init = String::new();
            let mut has_pic = false;
            let mut k = 0;
            while k < seg.len() {
                match seg[k].to_uppercase().as_str() {
                    "PIC" | "PICTURE" => {
                        has_pic = true;
                        let mut pc = seg.get(k + 1).cloned().unwrap_or_default();
                        if pc.to_uppercase() == "IS" {
                            pc = seg.get(k + 2).cloned().unwrap_or_default();
                        }
                        let u = pc.to_uppercase();
                        numeric = u.starts_with('9') || u.starts_with("S9") || u.starts_with("V9");
                        decimal = numeric && (u.contains('V') || u.contains('.'));
                    }
                    "VALUE" => {
                        let mut v = seg.get(k + 1).cloned().unwrap_or_default();
                        if v.to_uppercase() == "IS" {
                            v = seg.get(k + 2).cloned().unwrap_or_default();
                        }
                        init = match v.to_uppercase().as_str() {
                            "ZERO" | "ZEROS" | "ZEROES" => "0".into(),
                            "SPACE" | "SPACES" => "\"\"".into(),
                            _ => v,
                        };
                    }
                    _ => {}
                }
                k += 1;
            }
            if has_pic && name.to_uppercase() != "FILLER" && !p.vars.iter().any(|v| v.name == name.trim_end_matches('.')) {
                p.vars.push(Var { name: name.trim_end_matches('.').to_string(), numeric, init, decimal, undeclared: false });
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    // 手続き部
    if let Some(pa) = proc_at {
        let mut st = P { t: toks[pa + 2..].to_vec(), i: 0, ended: false, used: BTreeSet::new() };
        // PROCEDURE DIVISION [USING ...] . の見出しの残りを読み飛ばす
        if pa + 1 < toks.len() && !toks[pa + 1].dot {
            while let Some(t) = st.peek() {
                let d = t.dot;
                st.next();
                if d {
                    break;
                }
            }
        }
        st.ended = false;
        let mut cur: Option<(String, Vec<Stmt>)> = None;
        while st.peek().is_some() {
            // 段落見出し: 文頭の「語.」で、動詞ではないもの
            let t = st.peek().unwrap().clone();
            if t.dot && !is_starter(&t.s) && t.s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                if let Some(c) = cur.take() {
                    p.paragraphs.push(c);
                }
                st.next();
                st.ended = false;
                cur = Some((ident(&t.s), vec![]));
                continue;
            }
            st.ended = false;
            let before = st.i;
            let s = st.statement();
            if st.i == before {
                st.next();
            }
            if let Some(s) = s {
                match cur.as_mut() {
                    Some((_, v)) => v.push(s),
                    None => p.stmts.push(s),
                }
            }
        }
        if let Some(c) = cur.take() {
            p.paragraphs.push(c);
        }
        // 宣言の無い変数を足す(段落名は除く)
        let paras: Vec<String> = p.paragraphs.iter().map(|x| x.0.clone()).collect();
        for u in st.used {
            if !p.vars.iter().any(|v| ident(&v.name) == u) && !paras.contains(&u) {
                p.vars.push(Var { name: u, numeric: true, init: String::new(), decimal: false, undeclared: true });
            }
        }
    }
    p
}

// ---------------- 出力 ----------------

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Py,
    Rs,
}

fn expr(e: &Expr, l: Lang) -> String {
    match e {
        Expr::Num(n) => n.clone(),
        Expr::Str(s) => format!("{s:?}"),
        Expr::Var(v) => format!("self.{v}"),
        Expr::Neg(x) => format!("(-{})", expr(x, l)),
        Expr::Bin('^', a, b) => match l {
            Lang::Py => format!("({} ** {})", expr(a, l), expr(b, l)),
            Lang::Rs => format!("({}).pow(({}) as u32)", expr(a, l), expr(b, l)),
        },
        Expr::Bin('/', a, b) => match l {
            Lang::Py => format!("({} // {})", expr(a, l), expr(b, l)),
            Lang::Rs => format!("({} / {})", expr(a, l), expr(b, l)),
        },
        Expr::Bin(o, a, b) => format!("({} {o} {})", expr(a, l), expr(b, l)),
    }
}

fn cond(c: &Cond, l: Lang) -> String {
    match c {
        Cond::Cmp(o, a, b) => format!("{} {o} {}", expr(a, l), expr(b, l)),
        Cond::And(a, b) => format!("({} {} {})", cond(a, l), if l == Lang::Py { "and" } else { "&&" }, cond(b, l)),
        Cond::Or(a, b) => format!("({} {} {})", cond(a, l), if l == Lang::Py { "or" } else { "||" }, cond(b, l)),
        Cond::Not(a) => format!("{}({})", if l == Lang::Py { "not " } else { "!" }, cond(a, l)),
    }
}

fn is_num(p: &Program, name: &str) -> bool {
    p.vars.iter().any(|v| ident(&v.name) == name && v.numeric)
}

fn emit(p: &Program, ss: &[Stmt], l: Lang, ind: usize, o: &mut String) {
    let pad = "    ".repeat(ind);
    if ss.is_empty() {
        o.push_str(&format!("{pad}{}\n", if l == Lang::Py { "pass" } else { "" }));
        return;
    }
    for s in ss {
        match s {
            Stmt::Display(a) => match l {
                Lang::Py => o.push_str(&format!("{pad}self.out.append(\"\".join(str(x) for x in [{}]))\n", a.iter().map(|e| expr(e, l)).collect::<Vec<_>>().join(", "))),
                Lang::Rs => o.push_str(&format!("{pad}self.out += &format!(\"{}\\n\", {});\n", "{}".repeat(a.len()), a.iter().map(|e| expr(e, l)).collect::<Vec<_>>().join(", "))),
            },
            Stmt::Move(src, tg) => {
                for t in tg {
                    let e = expr(src, l);
                    match l {
                        Lang::Py => o.push_str(&format!("{pad}self.{t} = {e}\n")),
                        Lang::Rs => {
                            if is_num(p, t) {
                                o.push_str(&format!("{pad}self.{t} = {e};\n"));
                            } else {
                                o.push_str(&format!("{pad}self.{t} = {e}.to_string();\n"));
                            }
                        }
                    }
                }
            }
            Stmt::Assign(t, e) => o.push_str(&format!("{pad}self.{t} = {}{}\n", expr(e, l), if l == Lang::Rs { ";" } else { "" })),
            Stmt::AddTo(t, e) => o.push_str(&format!("{pad}self.{t} += {}{}\n", expr(e, l), if l == Lang::Rs { ";" } else { "" })),
            Stmt::SubFrom(t, e) => o.push_str(&format!("{pad}self.{t} -= {}{}\n", expr(e, l), if l == Lang::Rs { ";" } else { "" })),
            Stmt::MulBy(t, e) => o.push_str(&format!("{pad}self.{t} *= {}{}\n", expr(e, l), if l == Lang::Rs { ";" } else { "" })),
            Stmt::If(Cond::Cmp(o1, Expr::Num(a), Expr::Num(b)), body, _) if o1 == "==" && a == "1" && b == "1" => {
                // 複数代入先の束(ADD ... TO a b)
                emit(p, body, l, ind, o);
            }
            Stmt::If(c, th, el) => {
                match l {
                    Lang::Py => o.push_str(&format!("{pad}if {}:\n", cond(c, l))),
                    Lang::Rs => o.push_str(&format!("{pad}if {} {{\n", cond(c, l))),
                }
                emit(p, th, l, ind + 1, o);
                if !el.is_empty() {
                    o.push_str(&format!("{pad}{}\n", if l == Lang::Py { "else:" } else { "} else {" }));
                    emit(p, el, l, ind + 1, o);
                }
                if l == Lang::Rs {
                    o.push_str(&format!("{pad}}}\n"));
                }
            }
            Stmt::Perform(lp, body) => {
                match (lp, l) {
                    (Loop::Once, _) => emit(p, body, l, ind, o),
                    (Loop::Times(n), Lang::Py) => {
                        o.push_str(&format!("{pad}for _ in range({}):\n", expr(n, l)));
                        emit(p, body, l, ind + 1, o);
                    }
                    (Loop::Times(n), Lang::Rs) => {
                        o.push_str(&format!("{pad}for _ in 0..({}) {{\n", expr(n, l)));
                        emit(p, body, l, ind + 1, o);
                        o.push_str(&format!("{pad}}}\n"));
                    }
                    (Loop::Until(c), Lang::Py) => {
                        o.push_str(&format!("{pad}while not ({}):\n", cond(c, l)));
                        emit(p, body, l, ind + 1, o);
                    }
                    (Loop::Until(c), Lang::Rs) => {
                        o.push_str(&format!("{pad}while !({}) {{\n", cond(c, l)));
                        emit(p, body, l, ind + 1, o);
                        o.push_str(&format!("{pad}}}\n"));
                    }
                    (Loop::Varying { var, from, by, until }, Lang::Py) => {
                        o.push_str(&format!("{pad}self.{var} = {}\n{pad}while not ({}):\n", expr(from, l), cond(until, l)));
                        emit(p, body, l, ind + 1, o);
                        o.push_str(&format!("{pad}    self.{var} += {}\n", expr(by, l)));
                    }
                    (Loop::Varying { var, from, by, until }, Lang::Rs) => {
                        o.push_str(&format!("{pad}self.{var} = {};\n{pad}while !({}) {{\n", expr(from, l), cond(until, l)));
                        emit(p, body, l, ind + 1, o);
                        o.push_str(&format!("{pad}    self.{var} += {};\n{pad}}}\n", expr(by, l)));
                    }
                }
            }
            Stmt::Call(n) => match l {
                Lang::Py => o.push_str(&format!("{pad}self.p_{n}()\n")),
                Lang::Rs => o.push_str(&format!("{pad}if self.p_{n}() {{ return true; }}\n")),
            },
            Stmt::Stop => o.push_str(&match l {
                Lang::Py => format!("{pad}raise _Stop()\n"),
                Lang::Rs => format!("{pad}return true;\n"),
            }),
            Stmt::Nop => {}
            Stmt::Todo(t) => o.push_str(&format!("{pad}{} TODO(COBOL_to): 未対応の文 {t}\n", if l == Lang::Py { "#" } else { "//" })),
        }
    }
}

fn init_value(v: &Var) -> String {
    if v.init.is_empty() {
        if v.numeric { "0".into() } else { "\"\"".into() }
    } else {
        v.init.clone()
    }
}

pub fn to_python(p: &Program) -> String {
    let mut o = String::from("# COBOL_to generated(数値は整数として扱う。/ は切り捨て除算 //)\nclass _Stop(Exception):\n    pass\n\n\nclass State:\n    def __init__(self):\n        self.out = []\n");
    for v in &p.vars {
        o += &format!("        self.{} = {}{}\n", ident(&v.name), init_value(v), if v.decimal { "  # TODO(COBOL_to): 小数 PIC を整数として扱っている" } else if v.undeclared { "  # TODO(COBOL_to): 宣言が見つからない変数" } else { "" });
    }
    o += "\n    def main(self):\n";
    let mut body = String::new();
    emit(p, &p.stmts, Lang::Py, 2, &mut body);
    o += &body;
    for (n, _) in &p.paragraphs {
        o += &format!("        self.p_{n}()\n");
    }
    if p.stmts.is_empty() && p.paragraphs.is_empty() {
        o += "        pass\n";
    }
    for (n, ss) in &p.paragraphs {
        o += &format!("\n    def p_{n}(self):\n");
        emit(p, ss, Lang::Py, 2, &mut o);
    }
    o + "\n\ndef run() -> str:\n    s = State()\n    try:\n        s.main()\n    except _Stop:\n        pass\n    return \"\\n\".join(s.out) + \"\\n\"\n"
}

pub fn to_rust(p: &Program) -> String {
    let mut o = String::from("// COBOL_to generated. 数値は i64、文字は String。\n#![allow(unused_parens, unused_assignments, unused_mut, unused_variables, dead_code)]\npub struct State {\n    out: String,\n");
    for v in &p.vars {
        o += &format!("    {}: {},\n", ident(&v.name), if v.numeric { "i64" } else { "String" });
    }
    o += "}\n\nimpl State {\n    fn new() -> Self {\n        State {\n            out: String::new(),\n";
    for v in &p.vars {
        let init = init_value(v);
        o += &format!("            {}: {}, {}\n", ident(&v.name), if v.numeric { init } else { format!("{init}.to_string()") }, if v.decimal { "// TODO(COBOL_to): 小数 PIC を整数として扱っている" } else if v.undeclared { "// TODO(COBOL_to): 宣言が見つからない変数" } else { "" });
    }
    o += "        }\n    }\n\n    fn main(&mut self) -> bool {\n";
    emit(p, &p.stmts, Lang::Rs, 2, &mut o);
    for (n, _) in &p.paragraphs {
        o += &format!("        if self.p_{n}() {{ return true; }}\n");
    }
    o += "        false\n    }\n";
    for (n, ss) in &p.paragraphs {
        o += &format!("\n    fn p_{n}(&mut self) -> bool {{\n");
        emit(p, ss, Lang::Rs, 2, &mut o);
        o += "        false\n    }\n";
    }
    o + "}\n\npub fn run() -> String {\n    let mut s = State::new();\n    s.main();\n    s.out\n}\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_roundtrip() {
        let src = include_str!("../tests/samples/hello.cbl");
        let p = parse(src, false);
        assert_eq!(p.id, "HELLO");
        assert_eq!(p.vars.len(), 2);
        let r = to_rust(&p);
        assert!(r.contains("format!(\"{}{}\\n\", \"HELLO \", self.ws_name)"), "{r}");
        assert!(r.contains("self.ws_count += 2;"), "{r}");
        assert!(to_python(&p).contains("self.ws_count += 2"));
    }
    const LOOP: &str = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. LOOPY.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       01 I PIC 9(3) VALUE 0.\n       01 TOTAL PIC 9(5) VALUE 0.\n       01 MSG PIC X(10).\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           PERFORM VARYING I FROM 1 BY 1 UNTIL I > 5\n               ADD I TO TOTAL\n           END-PERFORM.\n           COMPUTE TOTAL = TOTAL * 2 + (I - 1).\n           IF TOTAL > 20 AND NOT TOTAL = 99\n               MOVE \"BIG\" TO MSG\n           ELSE\n               MOVE \"SMALL\" TO MSG\n           END-IF.\n           PERFORM SHOW 2 TIMES.\n           STOP RUN.\n       SHOW.\n           DISPLAY \"TOTAL=\" TOTAL \" MSG=\" MSG.\n";
    #[test]
    fn structured_program() {
        let p = parse(LOOP, false);
        assert_eq!(p.paragraphs.len(), 2);
        let py = to_python(&p);
        assert!(py.contains("while not (self.i > 5):"), "{py}");
        assert!(py.contains("self.total = ((self.total * 2) + (self.i - 1))"), "{py}");
        assert!(py.contains("if (self.total > 20 and not (self.total == 99)):"), "{py}");
        assert!(py.contains("for _ in range(2):"), "{py}");
        let rs = to_rust(&p);
        assert!(rs.contains("while !(self.i > 5) {"), "{rs}");
        assert!(rs.contains("fn p_show(&mut self) -> bool"), "{rs}");
    }
    #[test]
    fn evaluate_arms() {
        let src = "       PROGRAM-ID. E.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 N PIC 9 VALUE 2.
       PROCEDURE DIVISION.
           EVALUATE N
               WHEN 1 DISPLAY \"ONE\"
               WHEN 2 WHEN 3 DISPLAY \"TWO-THREE\"
               WHEN OTHER DISPLAY \"X\"
           END-EVALUATE.
           STOP RUN.
";
        let py = to_python(&parse(src, false));
        assert!(py.contains("if self.n == 1:") && py.contains("elif") == false && py.contains("self.n == 3"), "{py}");
    }
    #[test]
    fn unsupported_stays_visible() {
        let src = "       PROGRAM-ID. T.\n       PROCEDURE DIVISION.\n           OPEN INPUT F.\n           STOP RUN.\n";
        let p = parse(src, false);
        assert!(to_python(&p).contains("TODO(COBOL_to): 未対応の文 OPEN"));
    }
}
