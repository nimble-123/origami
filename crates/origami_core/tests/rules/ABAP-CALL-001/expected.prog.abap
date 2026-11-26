DATA(lv_result) = lo_calculator->calculate( iv_amount   = lv_amount
                                            iv_currency = lv_currency
                                            iv_date     = sy-datum
                                            iv_rate     = lv_exchange_rate ).
lo_log->add( iv_text = lv_text
             iv_type = 'E' ).
lo_log->add( iv_text ).
lv_text = condense( val = lv_text
                    del = ` ` ).
IF lo_check->is_valid( iv_id   = lv_id
                       iv_date = sy-datum ) = abap_true.
ENDIF.
lv_text = |{ lo_fmt->amount( iv_value = lv_amount iv_currency = lv_currency ) }|.
RAISE EXCEPTION TYPE zcx_flight
  EXPORTING textid = zcx_flight=>not_found
            iv_id  = lv_id.
RAISE EXCEPTION TYPE zcx_flight EXPORTING textid = zcx_flight=>not_found.
