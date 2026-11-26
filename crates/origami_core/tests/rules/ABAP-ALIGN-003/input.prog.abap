DATA: lv_count TYPE i,
 lt_items_with_long_name TYPE ty_items,
 lo_logger TYPE REF TO zif_logger.
TYPES: BEGIN OF ty_order,
 id TYPE i,
 BEGIN OF address,
 city TYPE string,
 zip TYPE c LENGTH 5,
 END OF address,
 customer TYPE string,
 END OF ty_order.
