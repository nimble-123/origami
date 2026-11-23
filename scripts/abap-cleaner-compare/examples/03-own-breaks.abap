lv_count =
  lines( lt_flights ).
lo_service->read(
  exporting
    iv_id = lv_id ).
mt_items = value #(
  ( id = 1 name = `Alpha` )
  ( id = 2 name = `Beta` ) ).
