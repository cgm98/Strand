//! Acceso a datos. Funciones libres sobre `&Connection`: las que necesitan
//! atomicidad abren su propia transacción; las demás pueden componerse
//! dentro de una transacción del llamador (p. ej. los importadores).

pub mod folders;
pub mod notes;
pub mod search;
pub mod tags;
pub mod trash;
