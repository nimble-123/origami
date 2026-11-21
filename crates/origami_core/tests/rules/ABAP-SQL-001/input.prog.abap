SELECT carrid, connid, fldate, price, currency FROM sflight WHERE carrid = @lv_carrier AND connid = @lv_connection INTO TABLE @DATA(lt_flights).
