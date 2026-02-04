use uuid::Uuid;

/// BLE characteristic used by DeTong / Detonger printers for writes (GATT).
pub const PRINTER_WRITE_CHARACTERISTIC_UUID: Uuid =
    uuid::uuid!("49535343-8841-43f4-a8d4-ecbe34729bb3");

