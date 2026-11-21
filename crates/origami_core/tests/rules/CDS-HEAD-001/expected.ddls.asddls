define root view entity ZI_Travel
  as select from ztravel as Travel
  composition [0..*] of ZI_Booking as _Booking
  association [0..1] to ZI_Agency as _Agency on $projection.AgencyID = _Agency.AgencyID
      and $projection.Client = _Agency.Client
{
  key travel_id as TravelID,
      _Booking
}
