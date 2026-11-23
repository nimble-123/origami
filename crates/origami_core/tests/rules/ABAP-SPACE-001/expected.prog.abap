REPORT zspace.
DATA: lv_a TYPE i, lv_b TYPE string.
DATA lo_any TYPE REF TO object ##NEEDED.
SELECT carrid, connid FROM spfli WHERE carrid IN ( 'AA' , 'LH' ) ORDER BY carrid, connid INTO TABLE @DATA(lt_conn).
lv_a = 42. " the answer
CLEAR: lv_a, " first item
       lv_b.
* comment line
WRITE / .
lv_a = 1 " stays
. " because of this comment
