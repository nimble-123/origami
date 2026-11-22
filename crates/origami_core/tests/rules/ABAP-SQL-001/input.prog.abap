SELECT carrid, connid, fldate, price, currency FROM sflight WHERE carrid = @lv_carrier AND connid = @lv_connection INTO TABLE @DATA(lt_flights).
SELECT SINGLE file_name mimetype FROM smimphf INTO CORRESPONDING FIELDS OF cs_extra WHERE langu = sy-langu AND loio_id = is_loio-objid.
UPDATE zflights SET price = @lv_new_price, currency = @lv_currency WHERE carrid = @lv_carrier AND connid = @lv_connection.
DELETE FROM zflights WHERE carrid = @lv_carrier AND connid = @lv_connection AND fldate < @lv_cutoff_date_for_the_deletion_run.
MODIFY lt_flights FROM ls_flight TRANSPORTING price currency WHERE carrid = lv_carrier AND connid = lv_connection AND fldate < lv_cutoff.
select single from sflight fields price where carrid = @lv_carrier into @data(lv_price).
