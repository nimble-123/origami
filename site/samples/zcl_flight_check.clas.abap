CLASS zcl_flight_check IMPLEMENTATION.
  METHOD check.
    DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum iv_mode = 'A' ).
    data lv_count type i.
    if ls_rule-active = abap_true and lv_count = 0 and is_flight-seatsmax > 0.
      rv_ok = abap_true.
    endif.
  ENDMETHOD.
ENDCLASS.
