lt_ids = VALUE #( FOR ls_item IN lt_items WHERE ( active = abap_true )
                  ( ls_item-id ) ).
lt_pairs = VALUE #( FOR ls_a IN lt_a FOR ls_b IN ls_a-items
                    ( a = ls_a-id b = ls_b-id ) ).
lt_index = VALUE #( FOR i = 1 UNTIL i > 3
                    ( i ) ).
lt_groups = VALUE #( FOR GROUPS grp OF ls_item IN lt_items GROUP BY ( cat = ls_item-cat )
                     ( grp ) ).
lv_sum = REDUCE i( INIT sum = 0
                   FOR ls_item IN lt_items
                   NEXT sum = sum + ls_item-amount ).
lv_text = REDUCE string( INIT t = `` sep = ``
                         FOR ls_item IN lt_items
                         NEXT t = t && sep && ls_item-name sep = `, ` ).
