lo_service->read( exporting iv_id = lv_id importing es_data = ls_data ).
lo_service->check( exporting iv_id = lv_id ).
data(lv_result) = lo_calculator->calculate( iv_amount = lv_amount iv_currency = lv_currency iv_date = sy-datum iv_rounding = abap_true ).
