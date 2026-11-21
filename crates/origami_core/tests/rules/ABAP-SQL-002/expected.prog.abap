WITH +carriers AS ( SELECT carrid, carrname FROM scarr ),
     +seats AS (
       SELECT carrid, connid, sum( seatsocc ) AS seats
           FROM sflight
           WHERE carrid BETWEEN 'AA' AND 'LH'
           GROUP BY carrid, connid
     )
SELECT * FROM +seats INTO TABLE @DATA(result).
OPEN CURSOR WITH HOLD @DATA(dbcur) FOR SELECT * FROM sflight WHERE carrid = 'LH'.
MERGE INTO ztarget AS a
    USING zsource AS b
    ON a~kf = b~kf
    WHEN MATCHED THEN UPDATE SET c1 = '#'
    WHEN NOT MATCHED THEN INSERT ALL.
