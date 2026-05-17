pub mod component_inventory;

pub use component_inventory::{
    AddCollectionComponentChange, AddCollectionComponentResult, BindArtifactChange,
    BindArtifactResult, CollectionRegistration, ComponentInventory, ComponentRegistration,
    ConfigureProviderChange, ConfigureProviderResult, ManagedCollection, RegisterCollectionChange,
    RegisterCollectionResult, RegisterComponentChange, RegisterComponentResult,
    RemoveCollectionComponentChange, RemoveCollectionComponentResult,
};
