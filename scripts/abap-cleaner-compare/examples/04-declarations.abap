data lv_count type i.
data lt_flights type standard table of sflight with empty key.
data lo_logger type ref to zcl_logger.
data: lv_a type i, lv_long_name type string.
types: begin of ty_item, id type i, name type string, end of ty_item.
