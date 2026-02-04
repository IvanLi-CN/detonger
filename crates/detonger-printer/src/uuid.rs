use uuid::Uuid;

/// BLE service used by DeTong / Detonger printers (GATT).
pub const PRINTER_SERVICE_UUID: Uuid = uuid::uuid!("49535343-fe7d-4ae5-8fa9-9fafd205e455");

/// BLE characteristic used by DeTong / Detonger printers for writes (GATT).
pub const PRINTER_WRITE_CHARACTERISTIC_UUID: Uuid =
    uuid::uuid!("49535343-8841-43f4-a8d4-ecbe34729bb3");

/// BLE characteristic used by DeTong / Detonger printers for notifications (GATT).
#[allow(dead_code)]
pub const PRINTER_NOTIFY_CHARACTERISTIC_UUID: Uuid =
    uuid::uuid!("49535343-1e4d-4bd9-ba61-23c647249616");
