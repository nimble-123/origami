IF lv_amount_with_a_rather_long_name > 0 AND lv_currency_with_a_rather_long_name IS NOT INITIAL AND lv_flag = abap_true.
  book( ).
ENDIF.
SELECT carrid, connid
  FROM spfli
  INTO TABLE @DATA(lt_routes)
  WHERE carrid = @ls_flight-carrid AND connid = @ls_flight-connid.
lv_result = lo_calculator->calculate_something_long( iv_amount )
  + lo_calculator->calculate_other( iv_currency )
  + lv_more.
