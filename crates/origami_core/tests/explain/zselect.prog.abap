REPORT zselect.
SELECT carrid, connid, fldate, price, currency FROM sflight WHERE carrid = @lv_carrid AND connid = @lv_connid INTO TABLE @DATA(lt_flights).
WITH +conns AS ( SELECT carrid, connid FROM spfli WHERE countryfr = 'DE' ) SELECT * FROM +conns INTO TABLE @DATA(lt_conns).
