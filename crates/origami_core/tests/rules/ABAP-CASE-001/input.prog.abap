data(lv_count) = lines( mt_items ).
mt_items = value #( ( id = 1 ) ).
select string_agg( name, ',' ) from t into @data(agg).
select var, median from t into table @lt.
ls_new = corresponding #( base ( in ) src ).
