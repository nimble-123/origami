lv_sign = COND #( WHEN iv_negative = abap_true THEN -1 ELSE 1 ).
lv_number = COND #( WHEN io_object->get_sub_object( )->get_number( ) <= if_object_boundaries=>co_maximum_number
                    THEN io_object->get_sub_object( )->get_number( )
                    ELSE if_object_boundaries=>co_maximum_number ).
lv_flag = COND #( WHEN lv_a = 1 THEN abap_true
                  WHEN lv_b = 2 THEN abap_false ).
lv_currency = COND #( WHEN ls_amounts-betrw <> 0           THEN ls_amounts-waers
                      WHEN ls_amounts-betrh IS NOT INITIAL THEN ls_amounts-hwaer
                      ELSE                                      gc_default_currency ).
lv_text = SWITCH string( sy-index
                         WHEN 1 THEN 'one'
                         WHEN 2 THEN 'two'
                         ELSE        THROW cx_overflow( ) ).
lv_num = SWITCH #( lv_num WHEN 999 THEN 0 ELSE lv_num + 1 ).
display_time_info( COND #( LET t = '120000' IN
                           WHEN sy-timlo < t THEN
                             |{ sy-timlo TIME = ISO } AM|
                           WHEN sy-timlo > t AND sy-timlo < '240000' THEN
                             |{ CONV t( sy-timlo - 12 * 3600 ) TIME = ISO } PM|
                           ELSE
                             THROW cx_cant_be( ) ) ).
