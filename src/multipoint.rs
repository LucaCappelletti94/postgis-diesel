use std::fmt::Debug;
use std::io::Cursor;

#[cfg(feature = "diesel")]
use crate::ewkb::{read_ewkb_header, write_ewkb_header};
use crate::{
    ewkb::{EwkbSerializable, GeometryType, BIG_ENDIAN},
    points::Dimension,
    types::*,
};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};

#[cfg(feature = "diesel")]
use crate::points::{read_point_coordinates, write_point};
use crate::sql_types::*;

impl<T> MultiPoint<T>
where
    T: PointT,
{
    pub fn new(srid: Option<u32>) -> Self {
        Self::with_capacity(srid, 0)
    }

    pub fn with_capacity(srid: Option<u32>, cap: usize) -> Self {
        MultiPoint {
            points: Vec::with_capacity(cap),
            srid,
        }
    }

    pub fn add_point(&mut self, point: T) -> &mut Self {
        self.points.push(point);
        self
    }

    pub fn add_points(&mut self, points: impl IntoIterator<Item = T>) -> &mut Self {
        for point in points {
            self.points.push(point);
        }
        self
    }

    pub fn dimension(&self) -> u32 {
        let mut dimension = Dimension::None as u32;
        if let Some(point) = self.points.first() {
            dimension |= point.dimension();
        }
        dimension
    }
}

impl<T> EwkbSerializable for MultiPoint<T>
where
    T: PointT,
{
    fn geometry_type(&self) -> u32 {
        let mut g_type = GeometryType::MultiPoint as u32;
        if let Some(point) = self.points.first() {
            g_type |= point.dimension();
        }
        g_type
    }
}

#[cfg(feature = "diesel")]
impl<T> diesel::deserialize::FromSql<Geometry, diesel::pg::Pg> for MultiPoint<T>
where
    T: PointT + Debug + Clone,
{
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        let mut r = Cursor::new(bytes.as_bytes());
        let end = r.read_u8()?;
        if end == BIG_ENDIAN {
            read_multipoint::<BigEndian, T>(&mut r)
        } else {
            read_multipoint::<LittleEndian, T>(&mut r)
        }
    }
}

#[cfg(feature = "diesel")]
impl<T> diesel::deserialize::FromSql<Geography, diesel::pg::Pg> for MultiPoint<T>
where
    T: PointT + Debug + Clone,
{
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        diesel::deserialize::FromSql::<Geometry, diesel::pg::Pg>::from_sql(bytes)
    }
}

#[cfg(feature = "diesel")]
impl<T> diesel::serialize::ToSql<Geometry, diesel::pg::Pg> for MultiPoint<T>
where
    T: PointT + Debug + EwkbSerializable,
{
    fn to_sql(
        &self,
        out: &mut diesel::serialize::Output<diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        write_multi_point(self, self.srid, out)
    }
}

#[cfg(feature = "diesel")]
impl<T> diesel::serialize::ToSql<Geography, diesel::pg::Pg> for MultiPoint<T>
where
    T: PointT + Debug + EwkbSerializable,
{
    fn to_sql(
        &self,
        out: &mut diesel::serialize::Output<diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        write_multi_point(self, self.srid, out)
    }
}

#[cfg(feature = "diesel")]
pub fn write_multi_point<T>(
    multipoint: &MultiPoint<T>,
    srid: Option<u32>,
    out: &mut diesel::serialize::Output<diesel::pg::Pg>,
) -> diesel::serialize::Result
where
    T: PointT + EwkbSerializable,
{
    write_ewkb_header(multipoint, srid, out)?;
    // size and points
    out.write_u32::<LittleEndian>(multipoint.points.len() as u32)?;
    for point in multipoint.points.iter() {
        write_point(point, None, out)?;
    }
    Ok(diesel::serialize::IsNull::No)
}

#[cfg(feature = "diesel")]
fn read_multipoint<T, P>(cursor: &mut Cursor<&[u8]>) -> diesel::deserialize::Result<MultiPoint<P>>
where
    T: byteorder::ByteOrder,
    P: PointT + Clone,
{
    let g_header = read_ewkb_header::<T>(cursor)?.expect(GeometryType::MultiPoint)?;
    read_multi_point_body::<T, P>(g_header.g_type, g_header.srid, cursor)
}

#[cfg(feature = "diesel")]
pub fn read_multi_point_body<T, P>(
    g_type: u32,
    srid: Option<u32>,
    cursor: &mut Cursor<&[u8]>,
) -> diesel::deserialize::Result<MultiPoint<P>>
where
    T: byteorder::ByteOrder,
    P: PointT + Clone,
{
    let len = cursor.read_u32::<T>()?;
    let mut mp = MultiPoint::with_capacity(srid, len as usize);
    for _i in 0..len {
        // skip 1 byte for byte order and 4 bytes for point type
        cursor.read_u8()?;
        cursor.read_u32::<T>()?;
        mp.add_point(read_point_coordinates::<T, P>(cursor, g_type, srid)?);
    }
    Ok(mp)
}
