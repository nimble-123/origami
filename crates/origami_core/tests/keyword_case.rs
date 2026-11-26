//! Keyword case (ABAP-CASE-001): every keyword of a recognized statement is
//! upper-cased, wherever it stands. Each case is written in lower case and
//! must format to the expected text without W003 (statement recognized).
//! The list grew from comparing origami with ABAP Cleaner on the test corpus
//! (scripts/abap-cleaner-compare/keywords.py). The cases are written in the
//! compact style, so their line breaks do not depend on the style rules.

use origami_core::{Code, LanguageHint, Options, Style, format};

fn check(cases: &[(&str, &str)]) {
    check_style(Style::Compact, "t.prog.abap", cases);
}

fn check_style(style: Style, path: &str, cases: &[(&str, &str)]) {
    let hint = LanguageHint::Path(path.into());
    let mut opts = Options::default();
    opts.lang.style = style;
    for (input, expected) in cases {
        for src in [input, expected] {
            let r = format(src.as_bytes(), &hint, &opts);
            assert!(!r.has(Code::W003), "not recognized: {src}");
            let out = String::from_utf8(r.output.expect("output")).unwrap();
            assert_eq!(&out, expected, "input: {src}");
        }
    }
}

#[test]
fn rap_derived_types() {
    check(&[
        (
            r#"data failed type response for failed early zi_travel.
"#,
            r#"DATA failed TYPE RESPONSE FOR FAILED EARLY zi_travel.
"#,
        ),
        (
            r#"data mapped type response for mapped late zi_travel.
"#,
            r#"DATA mapped TYPE RESPONSE FOR MAPPED LATE zi_travel.
"#,
        ),
        (
            r#"types tt_create type table for create zi_travel\_booking.
"#,
            r#"TYPES tt_create TYPE TABLE FOR CREATE zi_travel\_booking.
"#,
        ),
        (
            r#"data keys type table for read import zi_travel.
"#,
            r#"DATA keys TYPE TABLE FOR READ IMPORT zi_travel.
"#,
        ),
        (
            r#"data result type table for read result zi_travel.
"#,
            r#"DATA result TYPE TABLE FOR READ RESULT zi_travel.
"#,
        ),
        (
            r#"data link type table for read link zi_travel\_booking.
"#,
            r#"DATA link TYPE TABLE FOR READ LINK zi_travel\_booking.
"#,
        ),
        (
            r#"data act type table for action import zi_travel~accept.
"#,
            r#"DATA act TYPE TABLE FOR ACTION IMPORT zi_travel~accept.
"#,
        ),
        (
            r#"data val type table for validation zi_travel~check.
"#,
            r#"DATA val TYPE TABLE FOR VALIDATION zi_travel~check.
"#,
        ),
        (
            r#"data feat type table for instance features result zi_travel.
"#,
            r#"DATA feat TYPE TABLE FOR INSTANCE FEATURES RESULT zi_travel.
"#,
        ),
        (
            r#"data perm type structure for permissions request zi_travel.
"#,
            r#"DATA perm TYPE STRUCTURE FOR PERMISSIONS REQUEST zi_travel.
"#,
        ),
        (
            r#"data upd type table for update zi_travel.
"#,
            r#"DATA upd TYPE TABLE FOR UPDATE zi_travel.
"#,
        ),
        (
            r#"data: failed type response for failed zi_travel, reported type response for reported zi_travel.
"#,
            r#"DATA: failed TYPE RESPONSE FOR FAILED zi_travel, reported TYPE RESPONSE FOR REPORTED zi_travel.
"#,
        ),
        (
            r#"methods m importing keys type table for update zi_travel.
"#,
            r#"METHODS m IMPORTING keys TYPE TABLE FOR UPDATE zi_travel.
"#,
        ),
    ]);
}

/// RAP handler parameters in both styles; the aligned signature table once
/// left them in lower case.
#[test]
fn rap_handler_parameters() {
    let input = r#"class lhc definition inheriting from cl_abap_behavior_handler.
  private section.
    methods auth for instance authorization
      importing keys request requested for measure result result.
    methods glob for global authorization
      importing request requested for measure result result.
    methods accept for modify importing keys for action measure~accept result result.
    methods cba for read importing keys_rba for read measure\_items full requested result result link links.
endclass.
"#;
    let expected = r#"CLASS lhc DEFINITION INHERITING FROM cl_abap_behavior_handler.
  PRIVATE SECTION.
    METHODS auth FOR INSTANCE AUTHORIZATION IMPORTING keys REQUEST requested FOR measure RESULT result.
    METHODS glob FOR GLOBAL AUTHORIZATION IMPORTING REQUEST requested FOR measure RESULT result.
    METHODS accept FOR MODIFY IMPORTING keys FOR ACTION measure~accept RESULT result.
    METHODS cba FOR READ IMPORTING keys_rba FOR READ measure\_items FULL requested RESULT result LINK links.
ENDCLASS.
"#;
    for style in [Style::Aligned, Style::Compact] {
        check_style(style, "lhc.clas.abap", &[(input, expected)]);
    }
}

#[test]
fn inline_field_symbols() {
    check(&[
        (
            r#"loop at itab assigning field-symbol(<ls>).
endloop.
"#,
            r#"LOOP AT itab ASSIGNING FIELD-SYMBOL(<ls>).
ENDLOOP.
"#,
        ),
        (
            r#"read table itab assigning field-symbol(<ls>) index 1.
"#,
            r#"READ TABLE itab ASSIGNING FIELD-SYMBOL(<ls>) INDEX 1.
"#,
        ),
        (
            r#"assign itab[ 1 ] to field-symbol(<ls>).
"#,
            r#"ASSIGN itab[ 1 ] TO FIELD-SYMBOL(<ls>).
"#,
        ),
        (
            r#"insert value #( ) into table itab assigning field-symbol(<ls>).
"#,
            r#"INSERT VALUE #( ) INTO TABLE itab ASSIGNING FIELD-SYMBOL(<ls>).
"#,
        ),
        (
            r#"append initial line to itab assigning field-symbol(<ls>).
"#,
            r#"APPEND INITIAL LINE TO itab ASSIGNING FIELD-SYMBOL(<ls>).
"#,
        ),
    ]);
}

#[test]
fn others() {
    check(&[
        (
            r#"case x.
  when 1.
  when others.
endcase.
"#,
            r#"CASE x.
  WHEN 1.
  WHEN OTHERS.
ENDCASE.
"#,
        ),
        (
            r#"call function 'Z' exceptions not_found = 1 others = 2.
"#,
            r#"CALL FUNCTION 'Z' EXCEPTIONS not_found = 1 OTHERS = 2.
"#,
        ),
        (
            r#"lo->m( exceptions others = 1 ).
"#,
            r#"lo->m( EXCEPTIONS OTHERS = 1 ).
"#,
        ),
    ]);
}

#[test]
fn expressions() {
    check(&[
        (
            r#"cl_abap_unit_assert=>assert_true( xsdbool( lo is instance of zcl_x ) ).
"#,
            r#"cl_abap_unit_assert=>assert_true( xsdbool( lo IS INSTANCE OF zcl_x ) ).
"#,
        ),
        (
            r#"x = xsdbool( a is initial and b between 1 and 5 ).
"#,
            r#"x = xsdbool( a IS INITIAL AND b BETWEEN 1 AND 5 ).
"#,
        ),
        (
            r#"x = xsdbool( a is not initial or b is bound ).
"#,
            r#"x = xsdbool( a IS NOT INITIAL OR b IS BOUND ).
"#,
        ),
        (
            r#"x = itab[ key primary_key components id = 1 ]-name.
"#,
            r#"x = itab[ KEY primary_key COMPONENTS id = 1 ]-name.
"#,
        ),
        (
            r#"x = itab[ key sk id = 1 ]-name.
"#,
            r#"x = itab[ KEY sk id = 1 ]-name.
"#,
        ),
        (
            r#"x = cond #( when a = 1 then 2 else 3 ).
"#,
            r#"x = COND #( WHEN a = 1 THEN 2 ELSE 3 ).
"#,
        ),
        (
            r#"x = value #( for wa in itab where ( a = 1 ) ( wa ) ).
"#,
            r#"x = VALUE #( FOR wa IN itab WHERE ( a = 1 ) ( wa ) ).
"#,
        ),
        (
            r#"data(x) = reduce i( init s = 0 for wa in itab next s = s + wa ).
"#,
            r#"DATA(x) = REDUCE i( INIT s = 0 FOR wa IN itab NEXT s = s + wa ).
"#,
        ),
        (
            r#"x = reduce string( init t = `` for w in itab next t = cond #( when w = 1 then `a` else `b` ) ).
"#,
            r#"x = REDUCE string( INIT t = `` FOR w IN itab NEXT t = COND #( WHEN w = 1 THEN `a` ELSE `b` ) ).
"#,
        ),
        (
            r#"x = corresponding #( y mapping a = b except c ).
"#,
            r#"x = CORRESPONDING #( y MAPPING a = b EXCEPT c ).
"#,
        ),
        (
            r#"x = cast zcl_x( y ).
"#,
            r#"x = CAST zcl_x( y ).
"#,
        ),
        (
            r#"x = conv string( y ).
"#,
            r#"x = CONV string( y ).
"#,
        ),
        (
            r#"x = |{ ts timestamp = iso }|.
"#,
            r#"x = |{ ts TIMESTAMP = ISO }|.
"#,
        ),
        (
            r#"x = |a{ lv_d date = user } { lv_n width = 10 align = right pad = '0' }b|.
"#,
            r#"x = |a{ lv_d DATE = USER } { lv_n WIDTH = 10 ALIGN = RIGHT PAD = '0' }b|.
"#,
        ),
        (
            r#"x = |{ lv_x alpha = out case = upper }|.
"#,
            r#"x = |{ lv_x ALPHA = OUT CASE = UPPER }|.
"#,
        ),
    ]);
}

#[test]
fn classic_statements() {
    check(&[
        (
            r#"message e001(zmsg) with a b into data(lv_msg).
"#,
            r#"MESSAGE e001(zmsg) WITH a b INTO DATA(lv_msg).
"#,
        ),
        (
            r#"message id sy-msgid type sy-msgty number sy-msgno with sy-msgv1.
"#,
            r#"MESSAGE ID sy-msgid TYPE sy-msgty NUMBER sy-msgno WITH sy-msgv1.
"#,
        ),
        (
            r#"message lv_text type 'S' display like 'E'.
"#,
            r#"MESSAGE lv_text TYPE 'S' DISPLAY LIKE 'E'.
"#,
        ),
        (
            r#"message text-001 type 'I'.
"#,
            r#"MESSAGE TEXT-001 TYPE 'I'.
"#,
        ),
        (
            r#"message e001(zmsg) raising not_found.
"#,
            r#"MESSAGE e001(zmsg) RAISING not_found.
"#,
        ),
        (
            r#"read table itab with key id = 1 transporting no fields.
"#,
            r#"READ TABLE itab WITH KEY id = 1 TRANSPORTING NO FIELDS.
"#,
        ),
        (
            r#"data:lv_a type i.
"#,
            r#"DATA:lv_a TYPE i.
"#,
        ),
        (
            r#"condense lv_s no-gaps.
"#,
            r#"CONDENSE lv_s NO-GAPS.
"#,
        ),
        (
            r#"loop at screen.
endloop.
"#,
            r#"LOOP AT SCREEN.
ENDLOOP.
"#,
        ),
        (
            r#"select count( * ) from sflight into @data(n).
"#,
            r#"SELECT COUNT( * ) FROM sflight INTO @DATA(n).
"#,
        ),
        (
            r#"select carrid, cast( fltime as char( 6 ) ) as t from spfli into table @data(t).
"#,
            r#"SELECT carrid,
       CAST( fltime AS CHAR( 6 ) ) AS t
    FROM spfli
    INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"select carrid, max( price ) as max_price from sflight group by carrid into table @data(t).
"#,
            r#"SELECT carrid,
       MAX( price ) AS max_price
    FROM sflight
    GROUP BY carrid
    INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"select single @abap_true from sflight where exists ( select carrid from scarr where carrid = @lv_c ) into @data(ok).
"#,
            r#"SELECT SINGLE @abap_true FROM sflight WHERE EXISTS ( SELECT carrid FROM scarr WHERE carrid = @lv_c ) INTO @DATA(ok).
"#,
        ),
        (
            r#"select carrid, case when price > 100 then @abap_true else @abap_false end as high from sflight into table @data(t).
"#,
            r#"SELECT carrid,
       CASE WHEN price > 100 THEN @abap_true ELSE @abap_false END AS high
    FROM sflight
    INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"select a~carrid from scarr as a inner join spfli as b on a~carrid = b~carrid into table @data(t).
"#,
            r#"SELECT a~carrid FROM scarr AS a INNER JOIN spfli AS b ON a~carrid = b~carrid INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"insert lines of itab into table itab2.
"#,
            r#"INSERT LINES OF itab INTO TABLE itab2.
"#,
        ),
        (
            r#"insert zdb from table itab accepting duplicate keys.
"#,
            r#"INSERT zdb FROM TABLE itab ACCEPTING DUPLICATE KEYS.
"#,
        ),
        (
            r#"concatenate a b into c separated by space respecting blanks.
"#,
            r#"CONCATENATE a b INTO c SEPARATED BY space RESPECTING BLANKS.
"#,
        ),
        (
            r#"write / lv_x color col_negative.
"#,
            r#"WRITE / lv_x COLOR COL_NEGATIVE.
"#,
        ),
        (
            r#"types ty_t type sorted table of ty with unique key id.
"#,
            r#"TYPES ty_t TYPE SORTED TABLE OF ty WITH UNIQUE KEY id.
"#,
        ),
        (
            r#"types: begin of enum t_enum,
         a,
         b,
       end of enum t_enum.
"#,
            r#"TYPES: BEGIN OF ENUM t_enum,
         a,
         b,
       END OF ENUM t_enum.
"#,
        ),
        (
            r#"types: begin of ty_s.
  include type zs_x as x renaming with suffix _x.
types end of ty_s.
"#,
            r#"TYPES: BEGIN OF ty_s.
  INCLUDE TYPE zs_x AS x RENAMING WITH SUFFIX _x.
TYPES END OF ty_s.
"#,
        ),
    ]);
}

#[test]
fn more_statements() {
    check(&[
        (
            r#"commit work and wait.
"#,
            r#"COMMIT WORK AND WAIT.
"#,
        ),
        (
            r#"try.
    do( ).
  cleanup into data(lx).
endtry.
"#,
            r#"TRY.
    do( ).
  CLEANUP INTO DATA(lx).
ENDTRY.
"#,
        ),
        (
            r#"modify screen.
"#,
            r#"MODIFY SCREEN.
"#,
        ),
        (
            r#"read table itab into wa where a is initial.
"#,
            r#"READ TABLE itab INTO wa WHERE a IS INITIAL.
"#,
        ),
        (
            r#"events changed exporting value(ev_id) type i optional.
"#,
            r#"EVENTS changed EXPORTING VALUE(ev_id) TYPE i OPTIONAL.
"#,
        ),
        (
            r#"raise exception type zcx_x message id 'Z' type 'E' number '001' with a.
"#,
            r#"RAISE EXCEPTION TYPE zcx_x MESSAGE ID 'Z' TYPE 'E' NUMBER '001' WITH a.
"#,
        ),
        (
            r#"free memory id 'ABCD'.
"#,
            r#"FREE MEMORY ID 'ABCD'.
"#,
        ),
        (
            r#"call transformation id source data = ls_d result json lo_writer.
"#,
            r#"CALL TRANSFORMATION id SOURCE data = ls_d RESULT JSON lo_writer.
"#,
        ),
        (
            r#"create object lo_cut for testing.
"#,
            r#"CREATE OBJECT lo_cut FOR TESTING.
"#,
        ),
        (
            r#"class zbp_travel definition public abstract final for behavior of zi_travel.
endclass.
"#,
            r#"CLASS zbp_travel DEFINITION PUBLIC ABSTRACT FINAL FOR BEHAVIOR OF zi_travel.
ENDCLASS.
"#,
        ),
        (
            r#"data itab type standard table of i with empty key with non-unique sorted key sk components table_line.
"#,
            r#"DATA itab TYPE STANDARD TABLE OF i WITH EMPTY KEY WITH NON-UNIQUE SORTED KEY sk COMPONENTS table_line.
"#,
        ),
        (
            r#"types tt_ref type table of ref to i.
"#,
            r#"TYPES tt_ref TYPE TABLE OF REF TO i.
"#,
        ),
        (
            r#"types: begin of enum t_e base type i,
         a value is initial,
       end of enum t_e.
"#,
            r#"TYPES: BEGIN OF ENUM t_e BASE TYPE i,
         a VALUE IS INITIAL,
       END OF ENUM t_e.
"#,
        ),
        (
            r#"data req type request for change zi_travel.
"#,
            r#"DATA req TYPE REQUEST FOR CHANGE zi_travel.
"#,
        ),
        (
            r#"x = corresponding #( keys mapping from entity ).
"#,
            r#"x = CORRESPONDING #( keys MAPPING FROM ENTITY ).
"#,
        ),
        (
            r#"x = corresponding #( keys mapping to entity using control ).
"#,
            r#"x = CORRESPONDING #( keys MAPPING TO ENTITY USING CONTROL ).
"#,
        ),
        (
            r#"x = value #( for wa in itab from 2 to 5 ( wa ) ).
"#,
            r#"x = VALUE #( FOR wa IN itab FROM 2 TO 5 ( wa ) ).
"#,
        ),
        (
            r#"create object lo exceptions not_found = 1 others = 2.
"#,
            r#"CREATE OBJECT lo EXCEPTIONS not_found = 1 OTHERS = 2.
"#,
        ),
        (
            r#"methods m importing a type i optional b type i optional preferred parameter a.
"#,
            r#"METHODS m IMPORTING a TYPE i OPTIONAL b TYPE i OPTIONAL PREFERRED PARAMETER a.
"#,
        ),
        (
            r#"at selection-screen output.
"#,
            r#"AT SELECTION-SCREEN OUTPUT.
"#,
        ),
        (
            r#"x = cond #( when ( a <> 0 and b is initial ) or c = 1 then 1 else 2 ).
"#,
            r#"x = COND #( WHEN ( a <> 0 AND b IS INITIAL ) OR c = 1 THEN 1 ELSE 2 ).
"#,
        ),
        (
            r#"x = value #( ( id = 1 val = key ) ( id = 2 val = from ) ).
"#,
            r#"x = VALUE #( ( id = 1 val = key ) ( id = 2 val = from ) ).
"#,
        ),
        (
            r#"select count(*) from cross into @data(n).
"#,
            r#"SELECT COUNT(*) FROM cross INTO @DATA(n).
"#,
        ),
        (
            r#"selection-screen begin of block b01 with frame title text-b01.
"#,
            r#"SELECTION-SCREEN BEGIN OF BLOCK b01 WITH FRAME TITLE text-b01.
"#,
        ),
        (
            r#"x = value #( for groups group of wa in itab group by wa-a ( lines of value #( for m in group group ( m ) ) ) ).
"#,
            r#"x = VALUE #( FOR GROUPS group OF wa IN itab GROUP BY wa-a ( LINES OF VALUE #( FOR m IN GROUP group ( m ) ) ) ).
"#,
        ),
    ]);
}

#[test]
fn rarer_statements() {
    check(&[
        (
            r#"catch system-exceptions arithmetic_errors = 4 others = 8.
  lv_x = 1 / 0.
endcatch.
"#,
            r#"CATCH SYSTEM-EXCEPTIONS arithmetic_errors = 4 OTHERS = 8.
  lv_x = 1 / 0.
ENDCATCH.
"#,
        ),
        (
            r#"selection-screen skip.
"#,
            r#"SELECTION-SCREEN SKIP.
"#,
        ),
        (
            r#"selection-screen uline.
"#,
            r#"SELECTION-SCREEN ULINE.
"#,
        ),
        (
            r#"interfaces zif_x partially implemented.
"#,
            r#"INTERFACES zif_x PARTIALLY IMPLEMENTED.
"#,
        ),
        (
            r#"submit zprog with p_a eq p_a with s_b between 1 and 5 and return.
"#,
            r#"SUBMIT zprog WITH p_a EQ p_a WITH s_b BETWEEN 1 AND 5 AND RETURN.
"#,
        ),
        (
            r#"describe field <item> into td.
"#,
            r#"DESCRIBE FIELD <item> INTO td.
"#,
        ),
        (
            r#"select a from t into table @data(r) indicators null structure nulls.
"#,
            r#"SELECT a FROM t INTO TABLE @DATA(r) INDICATORS NULL STRUCTURE nulls.
"#,
        ),
        (
            r#"raise exception type zcx_x using message.
"#,
            r#"RAISE EXCEPTION TYPE zcx_x USING MESSAGE.
"#,
        ),
        (
            r#"select carrid, sum( price ) over( partition by carrid order by fldate rows between unbounded preceding and current row ) as s
    from sflight
    into table @data(t).
"#,
            r#"SELECT carrid,
       SUM( price ) OVER( PARTITION BY carrid ORDER BY fldate ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW ) AS s
    FROM sflight
    INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"select carrid, first from scarr order by carrid descending nulls first, first ascending nulls last into table @data(t).
"#,
            r#"SELECT carrid, first
    FROM scarr
    ORDER BY carrid DESCENDING NULLS FIRST,
             first ASCENDING NULLS LAST
    INTO TABLE @DATA(t).
"#,
        ),
        (
            r#"x = filter #( itab except where a >= 3 ).
"#,
            r#"x = FILTER #( itab EXCEPT WHERE a >= 3 ).
"#,
        ),
        (
            r#"x = filter #( itab except using key sk where a >= 3 ).
"#,
            r#"x = FILTER #( itab EXCEPT USING KEY sk WHERE a >= 3 ).
"#,
        ),
        (
            r#"create object type type cl_abap_elemdescr.
"#,
            r#"CREATE OBJECT type TYPE cl_abap_elemdescr.
"#,
        ),
        (
            r#"x = |{ text-003 } { conv string( y ) }|.
"#,
            r#"x = |{ TEXT-003 } { CONV string( y ) }|.
"#,
        ),
        (
            r#"x = itab[ table key primary_key components id = 1 ]-name.
"#,
            r#"x = itab[ TABLE KEY primary_key COMPONENTS id = 1 ]-name.
"#,
        ),
        (
            r#"x = cond #( when a = 1 then 1 else throw zcx_x( message e005(zmsg) with a ) ).
"#,
            r#"x = COND #( WHEN a = 1 THEN 1 ELSE THROW zcx_x( MESSAGE e005(zmsg) WITH a ) ).
"#,
        ),
        (
            r#"x = cond #( when a = 1 then 1 else throw zcx_x( using message ) ).
"#,
            r#"x = COND #( WHEN a = 1 THEN 1 ELSE THROW zcx_x( USING MESSAGE ) ).
"#,
        ),
        (
            r#"loop at itab into data(wa) group by ( key = wa-a gi = group index gs = group size ) into data(grp).
endloop.
"#,
            r#"LOOP AT itab INTO DATA(wa) GROUP BY ( key = wa-a gi = GROUP INDEX gs = GROUP SIZE ) INTO DATA(grp).
ENDLOOP.
"#,
        ),
        (
            r#"x = cond #( when a = 1 then |{ d date = iso }| else `` ).
"#,
            r#"x = COND #( WHEN a = 1 THEN |{ d DATE = ISO }| ELSE `` ).
"#,
        ),
        (
            r#"insert report lv_prog from lt_src state 'I'.
"#,
            r#"INSERT REPORT lv_prog FROM lt_src STATE 'I'.
"#,
        ),
        (
            r#"delete report lv_prog.
"#,
            r#"DELETE REPORT lv_prog.
"#,
        ),
        (
            r#"types ind_wa type zs_x with indicators comp_ind.
"#,
            r#"TYPES ind_wa TYPE zs_x WITH INDICATORS comp_ind.
"#,
        ),
        (
            r#"x = value #( for w in itab where ( oref is instance of zcl_x ) ( w ) ).
"#,
            r#"x = VALUE #( FOR w IN itab WHERE ( oref IS INSTANCE OF zcl_x ) ( w ) ).
"#,
        ),
        (
            r#"x = |{ boolc( a is not initial ) }|.
"#,
            r#"x = |{ boolc( a IS NOT INITIAL ) }|.
"#,
        ),
        (
            r#"x = |{ cond #( when a is not initial then `x` else `y` ) }|.
"#,
            r#"x = |{ COND #( WHEN a IS NOT INITIAL THEN `x` ELSE `y` ) }|.
"#,
        ),
        (
            r#"loop at group grp into data(member).
endloop.
"#,
            r#"LOOP AT GROUP grp INTO DATA(member).
ENDLOOP.
"#,
        ),
        (
            r#"loop at group into data(wa).
endloop.
"#,
            r#"LOOP AT group INTO DATA(wa).
ENDLOOP.
"#,
        ),
        (
            r#"x = value #( itab[ key id a = 1 ] optional ).
"#,
            r#"x = VALUE #( itab[ KEY id a = 1 ] OPTIONAL ).
"#,
        ),
        (
            r#"x = filter lcl=>tt_flights( itab using key sk where a = 1 ).
"#,
            r#"x = FILTER lcl=>tt_flights( itab USING KEY sk WHERE a = 1 ).
"#,
        ),
    ]);
}
