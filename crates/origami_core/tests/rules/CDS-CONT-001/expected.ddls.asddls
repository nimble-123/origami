define view entity ZI_X
  as select from t

  association [0..1] to ZI_Y as _Y on $projection.a = _Y.a

{
  key a,

      _Y
}
