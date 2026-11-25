define view entity ZI_Booking
  as select from /dmo/booking
  association [0..1] to /DMO/I_Travel as _Travel on $projection.TravelID = _Travel.TravelID
  association [0..*] to /DMO/I_Customer as _Customer on $projection.CustomerID = _Customer.CustomerID
{
  key travel_id as TravelID,
  key booking_id as BookingID,
      customer_id as CustomerID,
      carrier_id,
      _Travel.AgencyID as AgencyID,
      _Travel,
      _Customer
}
