use macroquad::{
    experimental::{
        collections::storage,
        scene::{self, Handle, RefMut},
    },
    prelude::*,
};

use crate::{NetSyncronizer, Pickup, Player, RemotePlayer, Resources};

pub struct GlobalEvents {
    last_spawn_time: f64,
    player: Handle<Player>,
    spawned_items: Vec<(usize, Handle<Pickup>)>,

    uid: usize,
    net_syncronizer: Handle<NetSyncronizer>,
}

impl GlobalEvents {
    const SPAWN_INTERVAL: f32 = 2.0;

    pub fn new(player: Handle<Player>, net_syncronizer: Handle<NetSyncronizer>) -> GlobalEvents {
        GlobalEvents {
            player,
            net_syncronizer,
            last_spawn_time: 0.0,
            uid: 0,
            spawned_items: vec![],
        }
    }
}

impl scene::Node for GlobalEvents {
    fn update(mut node: RefMut<Self>) {
        let mut net_syncronizer = scene::get_node(node.net_syncronizer).unwrap();

        if net_syncronizer.is_host() == false {
            return;
        }

        if get_time() - node.last_spawn_time >= Self::SPAWN_INTERVAL as _
            && node.spawned_items.len() < 3
        {
            let resources = storage::get::<Resources>().unwrap();

            let tilewidth = resources.tiled_map.raw_tiled_map.tilewidth as f32;
            let w = resources.tiled_map.raw_tiled_map.width as f32;
            let tileheight = resources.tiled_map.raw_tiled_map.tileheight as f32;
            let h = resources.tiled_map.raw_tiled_map.height as f32;

            let pos = loop {
                let x = rand::gen_range(0, w as i32) as f32;
                let y = rand::gen_range(0, h as i32 - 6) as f32;

                let pos = vec2((x + 0.5) * tilewidth, (y - 0.5) * tileheight);
                if resources
                    .collision_world
                    .collide_solids(pos, tilewidth as _, tileheight as _)
                    == false
                    && resources.collision_world.collide_solids(
                        pos,
                        tilewidth as _,
                        tileheight as i32 * 3,
                    )
                {
                    break pos;
                }
            };

            node.last_spawn_time = get_time();

            let item_id = node.uid;
            node.spawned_items
                .push((item_id, scene::add_node(Pickup::new(pos))));
            net_syncronizer.spawn_item(item_id, pos);

            node.uid += 1;
        }

        let mut player = scene::get_node(node.player).unwrap();
        let mut others = scene::find_nodes_by_type::<RemotePlayer>();

        node.spawned_items.retain(|(id, item_handle)| {
            let item = scene::get_node(*item_handle);
            // already destroyed itself
            if item.is_none() {
                net_syncronizer.delete_item(*id);
                return false;
            }
            let item = item.unwrap();

            let collide = |player: Vec2, pickup: Vec2| {
                (player + vec2(16., 32.)).distance(pickup + vec2(16., 16.)) < 60.
            };

            if collide(player.pos(), item.pos) {
                player.pick_weapon();
                item.delete();

                net_syncronizer.delete_item(*id);
                net_syncronizer.pick_up_item(*id, None);
                return false;
            }

            let other = others.find(|other| collide(other.pos(), item.pos));

            if let Some(other) = other {
                item.delete();

                net_syncronizer.delete_item(*id);
                net_syncronizer.pick_up_item(*id, Some(&other.network_id));
                return false;
            }

            return true;
        });
    }
}
