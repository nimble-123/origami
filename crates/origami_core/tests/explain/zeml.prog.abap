REPORT zeml.
READ ENTITIES OF /dmo/i_travel_m ENTITY travel FIELDS ( travel_id agency_id customer_id ) WITH VALUE #( ( travel_id = lv_id ) ) RESULT DATA(lt_travel).
