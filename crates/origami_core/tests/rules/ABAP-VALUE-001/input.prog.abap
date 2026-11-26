mt_items = VALUE #( ( id = 1 name = `Alpha` ) ( id = 2 name = `Beta` ) ( id = 3 name = `Gamma` ) ( id = 4 name = `Delta` ) ).
ls_flight = VALUE #( carrier_id = 'LH' flight_id = '0400' ).
ls_flight = VALUE #( carrier_id = 'LH' ).
mt_items = VALUE #( BASE mt_items ( id = 5 name = `Epsilon` ) ).
mt_ids = VALUE #( ( 1 ) ( 2 ) ( 3 ) ).
mt_items = VALUE #( ( id = 6 name = `Zeta` ) ).
lr_range = VALUE #( sign = 'I' option = 'EQ' ( low = 1 ) ( low = 2 ) ).
lo_item = NEW lcl_item( iv_id = 1 iv_name = `Eta` ).
DATA(ls_item) = mt_items[ id = 1 name = `Alpha` ].
DATA(ls_by_key) = mt_items[ KEY by_name COMPONENTS name = `Alpha` id = 1 ].
DATA(ls_first) = mt_items[ 1 ].
