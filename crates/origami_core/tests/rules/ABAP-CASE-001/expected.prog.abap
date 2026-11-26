DATA(lv_count) = lines( mt_items ).
mt_items = VALUE #( ( id = 1 ) ).
SELECT STRING_AGG( name, ',' ) FROM t INTO @DATA(agg).
SELECT var, median FROM t INTO TABLE @lt.
ls_new = CORRESPONDING #( BASE ( in ) src ).
