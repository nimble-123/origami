define view entity ZI_X as select from t
{
key uuid as TravelUUID,
@Semantics.text: true
descr as Description,
_Booking
// after the last element
}
