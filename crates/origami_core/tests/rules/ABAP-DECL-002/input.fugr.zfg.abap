DATA: BEGIN OF status_view. "state vector
          INCLUDE STRUCTURE vimstatus.
DATA: END OF status_view.
TYPES: BEGIN OF ty_item,
         id TYPE i.
TYPES: END OF ty_item.
TYPES: BEGIN OF ty_order,
  id TYPE i.
  INCLUDE TYPE ty_address.
TYPES END OF ty_order.
TYPES BEGIN OF ty_line.
TYPES id TYPE i.
TYPES name TYPE string.
TYPES END OF ty_line.
