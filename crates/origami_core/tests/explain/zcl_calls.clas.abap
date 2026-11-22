CLASS zcl_calls DEFINITION PUBLIC FINAL CREATE PUBLIC.
  PUBLIC SECTION.
    METHODS run IMPORTING iv_id TYPE i iv_date TYPE d iv_mode TYPE abap_bool iv_flag TYPE abap_bool iv_x TYPE i RETURNING VALUE(rv_ok) TYPE abap_bool.
ENDCLASS.

CLASS zcl_calls IMPLEMENTATION.
  METHOD run.
    DATA lv_a TYPE i. DATA lv_b TYPE i.
    rv_ok = zcl_check=>validate( iv_id = iv_id iv_date = iv_date iv_mode = iv_mode iv_flag = abap_false iv_x = 1 iv_y = 2 iv_z = 3 ).
    lt_rows = VALUE #( ( id = 1 name = 'first row with a long name' ) ( id = 2 name = 'second row with a long name' ) ( id = 3 name = 'third' ) ).
    IF iv_id > 0 AND iv_date IS NOT INITIAL AND iv_mode = abap_true AND lv_a = lv_b AND rv_ok = abap_false AND lv_b <> 42.
      CLEAR rv_ok.
    ENDIF.
  ENDMETHOD.
ENDCLASS.
