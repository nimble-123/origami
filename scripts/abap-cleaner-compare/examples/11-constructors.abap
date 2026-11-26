ls_flight = VALUE #( carrier_id = 'LH' connection_id = '0400' flight_date = sy-datum ).
mt_items = VALUE #( ( id = 1 name = `Alpha` ) ( id = 2 name = `Beta` ) ).
mt_items = VALUE #( BASE mt_items ( id = 3 name = `Gamma` ) ).
mt_ids = VALUE #( FOR ls_item IN mt_items WHERE ( id > 1 ) ( ls_item-id ) ).
lv_total = REDUCE i( INIT sum = 0 FOR ls_item IN mt_items NEXT sum = sum + ls_item-id ).
DATA(ls_alpha) = mt_items[ id = 1 name = `Alpha` ].
lo_log = NEW zcl_log( iv_object = 'ZFLIGHT' iv_subobject = 'BOOKING' ).
SORT mt_items BY name ASCENDING id DESCENDING.
TRY.
    lo_log->save( ).
  CATCH zcx_log_full zcx_log_locked INTO DATA(lx_error).
    CLEAR: ls_flight, mt_items.
ENDTRY.
