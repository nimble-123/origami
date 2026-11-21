//! Block pairs and the `SELECT`/`WITH` loop predicate.
//!
//! An opener matches only at the start of a statement. Every row is data
//! except [`select_is_loop`]. An E004 on valid code is a bug in this file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Opener(&'static str),
    Intermediate,
    Closer,
}

/// Closers in upper case.
const CLOSERS: &[&str] = &[
    "ENDCLASS",
    "ENDINTERFACE",
    "ENDMETHOD",
    "ENDFUNCTION",
    "ENDFORM",
    "ENDMODULE",
    "ENDIF",
    "ENDCASE",
    "ENDLOOP",
    "ENDDO",
    "ENDWHILE",
    "ENDTRY",
    "ENDCATCH",
    "ENDAT",
    "ENDSELECT",
    "ENDWITH",
    "ENDPROVIDE",
    "ENDON",
    "END-TEST-SEAM",
    "END-TEST-INJECTION",
    "ENDENHANCEMENT",
    "END-ENHANCEMENT-SECTION",
    "END-OF-DEFINITION",
    "ENDEXEC",
    "ENDCHAIN",
];

/// Classifies a statement. `words` are the statement's words in upper case
/// and `depth0` is the same list restricted to bracket depth 0.
pub fn classify(words: &[String], depth0: &[String]) -> Option<Role> {
    let w = |k: usize| words.get(k).map(String::as_str).unwrap_or("");
    let has = |x: &str| words.iter().any(|y| y == x);
    let first = w(0);
    if CLOSERS.contains(&first) {
        return Some(Role::Closer);
    }
    let opener = |c: &'static str| Some(Role::Opener(c));
    match first {
        "CLASS" => {
            let local_friends = words
                .windows(2)
                .any(|p| p[0] == "LOCAL" && p[1] == "FRIENDS");
            if has("DEFERRED") || has("LOAD") || local_friends {
                None
            } else if has("DEFINITION") || has("IMPLEMENTATION") {
                opener("ENDCLASS")
            } else {
                None
            }
        }
        "INTERFACE" => {
            if has("DEFERRED") || has("LOAD") {
                None
            } else {
                opener("ENDINTERFACE")
            }
        }
        "METHOD" => opener("ENDMETHOD"),
        "FUNCTION" => opener("ENDFUNCTION"),
        "FORM" => opener("ENDFORM"),
        "MODULE" => opener("ENDMODULE"),
        "IF" => opener("ENDIF"),
        "CASE" => opener("ENDCASE"),
        "LOOP" => opener("ENDLOOP"),
        // Dynpro flow logic.
        "CHAIN" => opener("ENDCHAIN"),
        "DO" => opener("ENDDO"),
        "WHILE" => opener("ENDWHILE"),
        "TRY" => opener("ENDTRY"),
        "CATCH" if w(1) == "SYSTEM-EXCEPTIONS" => opener("ENDCATCH"),
        // Control levels (`AT NEW f`) and extract field groups (`AT fg`,
        // `AT fg WITH fg2`) are blocks; event keywords (`AT SELECTION-SCREEN`,
        // `AT LINE-SELECTION`, `AT USER-COMMAND`, `AT PFnn`) are not.
        "AT" if !(matches!(w(1), "SELECTION-SCREEN" | "LINE-SELECTION" | "USER-COMMAND")
            || (w(1).starts_with("PF") && w(1)[2..].parse::<u8>().is_ok())) =>
        {
            opener("ENDAT")
        }
        "SELECT" if select_is_loop(depth0) => opener("ENDSELECT"),
        "WITH" if select_is_loop(depth0) => opener("ENDWITH"),
        "PROVIDE" => opener("ENDPROVIDE"),
        "ON" if w(1) == "CHANGE" => opener("ENDON"),
        "TEST-SEAM" => opener("END-TEST-SEAM"),
        "TEST-INJECTION" => opener("END-TEST-INJECTION"),
        "ENHANCEMENT" => opener("ENDENHANCEMENT"),
        "ENHANCEMENT-SECTION" => opener("END-ENHANCEMENT-SECTION"),
        "DEFINE" => opener("END-OF-DEFINITION"),
        "EXEC" if w(1) == "SQL" => opener("ENDEXEC"),
        "ELSE" | "ELSEIF" | "WHEN" | "CATCH" | "CLEANUP" => Some(Role::Intermediate),
        "PUBLIC" | "PROTECTED" | "PRIVATE" if w(1) == "SECTION" => Some(Role::Intermediate),
        _ => None,
    }
}

const AGGREGATES: &[&str] = &["COUNT", "SUM", "MIN", "MAX", "AVG", "COUNT(*)"];

/// True when a `SELECT` (or the main query of a `WITH`) needs `ENDSELECT`.
/// `depth0` holds the statement's words at bracket depth 0, in upper case.
pub fn select_is_loop(depth0: &[String]) -> bool {
    let Some(sel) = depth0.iter().position(|w| w == "SELECT") else {
        return false;
    };
    let q = &depth0[sel..];
    if q.iter().any(|w| w == "SINGLE") {
        return false;
    }
    let Some(into) = q.iter().position(|w| w == "INTO" || w == "APPENDING") else {
        // Obsolete short form with the `TABLES` work area
        // (`SELECT * FROM dbtab WHERE … . … ENDSELECT.`): a loop unless it
        // only aggregates.
        return sel == 0 && !is_aggregate_only(q);
    };
    let target = q[into + 1..]
        .iter()
        .find(|w| !matches!(w.as_str(), "CORRESPONDING" | "FIELDS" | "OF"));
    if target.map(String::as_str) == Some("TABLE") {
        return q.windows(2).any(|p| p[0] == "PACKAGE" && p[1] == "SIZE");
    }
    let has_group_by = q.windows(2).any(|p| p[0] == "GROUP" && p[1] == "BY");
    !is_aggregate_only(q) || has_group_by
}

/// True when the select list holds only aggregate functions (with aliases).
fn is_aggregate_only(q: &[String]) -> bool {
    // Old syntax: SELECT <list> FROM. New syntax: SELECT FROM … FIELDS <list>.
    let list: &[String] = if q.get(1).map(String::as_str) == Some("FROM") {
        match q.iter().position(|w| w == "FIELDS") {
            Some(f) => {
                let end = q[f + 1..]
                    .iter()
                    .position(|w| {
                        matches!(
                            w.as_str(),
                            "INTO" | "APPENDING" | "WHERE" | "GROUP" | "ORDER" | "HAVING" | "UNION"
                        )
                    })
                    .map(|p| f + 1 + p)
                    .unwrap_or(q.len());
                &q[f + 1..end]
            }
            None => return false,
        }
    } else {
        let end = q
            .iter()
            .position(|w| matches!(w.as_str(), "FROM" | "INTO" | "APPENDING"))
            .unwrap_or(q.len());
        &q[1..end]
    };
    let mut items = Vec::new();
    let mut k = 0;
    while k < list.len() {
        match list[k].as_str() {
            "DISTINCT" => k += 1,
            "AS" => k += 2,
            w => {
                items.push(w.trim_end_matches(','));
                k += 1;
            }
        }
    }
    !items.is_empty() && items.iter().all(|w| AGGREGATES.contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace()
            .map(|w| w.to_ascii_uppercase())
            .collect()
    }

    #[test]
    fn select_loop_forms() {
        assert!(select_is_loop(&words("SELECT * FROM t INTO wa")));
        assert!(select_is_loop(&words("SELECT * FROM t INTO @DATA ( wa )")));
        assert!(!select_is_loop(&words("SELECT SINGLE * FROM t INTO wa")));
        assert!(!select_is_loop(&words("SELECT * FROM t INTO TABLE itab")));
        assert!(select_is_loop(&words(
            "SELECT * FROM t INTO TABLE itab PACKAGE SIZE 100"
        )));
        assert!(!select_is_loop(&words("SELECT COUNT FROM t INTO lv")));
        assert!(!select_is_loop(&words("SELECT MAX AS m FROM t INTO lv")));
        assert!(!select_is_loop(&words(
            "SELECT MAX INTO lv FROM t WHERE a = 1"
        )));
        assert!(select_is_loop(&words(
            "SELECT COUNT FROM t INTO lv GROUP BY carrid"
        )));
        assert!(!select_is_loop(&words(
            "SELECT FROM t FIELDS COUNT INTO @lv"
        )));
        assert!(!select_is_loop(&words("SELECT COUNT FROM t WHERE x = 1")));
        assert!(!select_is_loop(&words(
            "SELECT * FROM t APPENDING CORRESPONDING FIELDS OF TABLE itab"
        )));
    }

    #[test]
    fn class_forms() {
        let c = |s: &str| classify(&words(s), &words(s));
        assert_eq!(c("CLASS lcl DEFINITION"), Some(Role::Opener("ENDCLASS")));
        assert_eq!(c("CLASS lcl DEFINITION DEFERRED"), None);
        assert_eq!(c("CLASS lcl DEFINITION LOCAL FRIENDS ltc"), None);
        assert_eq!(c("AT SELECTION-SCREEN"), None);
        assert_eq!(c("AT NEW carrid"), Some(Role::Opener("ENDAT")));
        assert_eq!(c("CATCH cx_root"), Some(Role::Intermediate));
        assert_eq!(
            c("CATCH SYSTEM-EXCEPTIONS x = 1"),
            Some(Role::Opener("ENDCATCH"))
        );
    }
}
