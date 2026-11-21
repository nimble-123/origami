with +carriers as ( select carrid, carrname from scarr ), +seats as ( select carrid, connid, sum( seatsocc ) as seats from sflight where carrid between 'AA' and 'LH' group by carrid, connid ) select * from +seats into table @data(result).
open cursor with hold @data(dbcur) for select * from sflight where carrid = 'LH'.
merge into ztarget as a using zsource as b on a~kf = b~kf when matched then update set c1 = '#' when not matched then insert all.
