use std::collections::HashMap;
use crate::resolver::EntityInfo;
use crate::select::FileInfo;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "#type")]
pub enum Hierarchy {
    Root(HashMap<String, Hierarchy>),
    Package(HashMap<String, Hierarchy>),
    Entity(EntityInfo)
}


pub fn build_hierarchy(files: Vec<FileInfo>) -> anyhow::Result<Hierarchy> {
    let mut hierarchy = Hierarchy::Root(HashMap::new());
    for file in files {
        let path = file.package.split('.').collect::<Vec<_>>();
        let _ = fill_hierarchy(&mut hierarchy, file.clone(), path);
    }
    Ok(hierarchy)
}


fn fill_hierarchy(hierarchy: &mut Hierarchy,
                  file: FileInfo, 
                  path: Vec<&str>) -> anyhow::Result<()> {
    match hierarchy {
        Hierarchy::Root(map) => {
            if path.len() > 0 { 
                if !map.contains_key(path[0]) {
                    map.insert(path[0].to_string(), Hierarchy::Package(HashMap::new()));
                }
                fill_hierarchy(
                    &mut map.get_mut(path[0]).unwrap(), file.clone(), path[1..].to_vec()
                )?;
            } else {
                for entity in file.entities {
                    map.insert(entity.name.clone(), Hierarchy::Entity(entity));
                }
            }
        }
        Hierarchy::Package(map) => {
            if path.len() > 0 {
                if !map.contains_key(path[0]) {
                    map.insert(path[0].to_string(), Hierarchy::Package(HashMap::new()));
                }
                fill_hierarchy(
                    &mut map.get_mut(path[0]).unwrap(), file.clone(), path[1..].to_vec()
                )?;
            } else {
                for entity in file.entities {
                    map.insert(entity.name.clone(), Hierarchy::Entity(entity));
                }
            }
        }
        Hierarchy::Entity(_) => {
            return Err(anyhow::anyhow!("Unexpected entity in hierarchy"));
        }
    }
    Ok(())
}