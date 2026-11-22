CLASS zcl_flight_check DEFINITION PUBLIC FINAL CREATE PUBLIC.
  PUBLIC SECTION.
    METHODS check IMPORTING is_flight TYPE sflight RETURNING VALUE(rv_ok) TYPE abap_bool.
ENDCLASS.

CLASS zcl_flight_check IMPLEMENTATION.
  METHOD check.
    DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum ).
    select carrid, connid from sflight where carrid = @is_flight-carrid into table @data(lt_flights).
    rv_ok = xsdbool( lines( lt_flights ) > 0 AND ls_rule-active = abap_true ).
  ENDMETHOD.
ENDCLASS.
