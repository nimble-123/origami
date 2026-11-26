define view entity ZI_X as select from t
association [0..1] to ZI_Y as _Y
on $projection.a = _Y.a
{
key a,
cast( b /* c */   
		as abap.char(1) )
	as B,
cast( b // x /* y
		as abap.char(1) )
	as C,
_Y
}
