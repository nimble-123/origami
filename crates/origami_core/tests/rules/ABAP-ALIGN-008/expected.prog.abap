REPORT zdemo.
DATA lv_count   TYPE i           VALUE 1.
DATA lt_flights TYPE STANDARD TABLE OF sflight WITH EMPTY KEY.
" logging
DATA lv_text    TYPE c LENGTH 10 VALUE 'abc'.
DATA lv_rate    TYPE p DECIMALS 2.
CONSTANTS lc_max  TYPE i      VALUE 10.
CONSTANTS lc_name TYPE string VALUE `origami`.
DATA mv_id    TYPE i         READ-ONLY.
DATA mv_count TYPE i VALUE 3 READ-ONLY.
