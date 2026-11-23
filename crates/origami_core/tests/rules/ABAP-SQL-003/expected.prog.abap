SELECT carrid, connid, fldate FROM sflight INTO TABLE @DATA(lt_flights).
SELECT carrid,
       connid,
       fldate,
       price,
       currency,
       planetype,
       seatsmax,
       seatsocc,
       paymentsum,
       seatsmax_b,
       seatsocc_b,
       seatsmax_f,
       seatsocc_f
    FROM sflight
    INTO TABLE @DATA(lt_all).
SELECT t1~carrid,
       t1~connid          AS connection,
       SUM( t2~seatsocc ) AS seats
    FROM spfli AS t1
    INNER JOIN sflight AS t2 ON t1~carrid = t2~carrid
    GROUP BY t1~carrid,
             t1~connid
    ORDER BY t1~carrid DESCENDING,
             t1~connid
    INTO TABLE @DATA(lt_seats).
SELECT FROM sflight
    FIELDS carrid,
           CASE WHEN price > 100 THEN @abap_true ELSE @abap_false END AS high
    INTO TABLE @DATA(lt_high).
SELECT SINGLE a~matnr
              b~maktx
    FROM mara AS a
    INNER JOIN makt AS b ON a~matnr = b~matnr
    INTO (lv_matnr, lv_maktx).
SELECT SINGLE carrid, connid
    FROM spfli
    WHERE carrid = @lv_carrier
    INTO ( @DATA(lv_carrier_id), @DATA(lv_connection_id) ).
