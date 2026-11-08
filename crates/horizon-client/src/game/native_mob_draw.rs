//! Mobs drawn by native mods on the client: `native_mob_submit` (vanilla's
//! `EntityRenderer.submit` for a mob, NeoForge's `RenderLivingEvent.Pre` when
//! it cancels) gets the mob's render state and a [`BlockEntityDraw`] posed at
//! its feet and turned with its body, so a mod draws a model of its own with
//! the helpers block entities draw with (`native_draw_cube`,
//! `native_draw_quad`, `native_draw_block`, `native_draw_item`). A player
//! has `native_player_submit` (`RenderPlayerEvent.Pre`), and every mob and
//! player in view gets `native_mob_layer` after its vanilla model
//! (`EntityRenderersEvent.AddLayers`), in the same frame.

use super::{Dim, Game, native_block_entity_draw::BlockEntityDraw};
use glam::{DVec3, Mat4};
use horizon_client::{
    block_entity_render::diffuse_lights,
    mesher::Vertex,
    mobs::{Mob, anim},
};

/// One mob's frame, as `EntityRenderer.extractRenderState` fills a
/// `LivingEntityRenderState`: what a model reads to pose and animate itself.
/// Angles are in degrees, interpolated to the frame's partial tick.
// Native mods read it; nothing in Horizon does.
#[derive(Clone, Debug)]
pub struct NativeMobRender<'a> {
    /// The mob's network id, as the mob helpers and mod messages name it.
    pub id: u32,
    /// Its entity type (`minecraft:villager`).
    pub kind: &'a str,
    /// Its `CustomName` as plain text, when it has one.
    pub custom_name: Option<&'a str>,
    /// `isBaby`.
    pub baby: bool,
    /// The frame's partial tick, 0 to 1.
    pub partial: f32,
    /// `ageInTicks`: ticks lived plus the partial tick, for idle motion.
    pub age: f32,
    /// `bodyRot`: the way the body faces (0 is south, 90 west).
    pub body_yaw: f32,
    /// `yRot`: the head's turn from the body, -180 to 180.
    pub head_yaw: f32,
    /// `xRot`: the head's pitch, positive looking down.
    pub pitch: f32,
    /// `walkAnimationPos`: how far the legs have swung, for a walk cycle
    /// (a vanilla quadruped's legs swing `cos(pos * 0.6662) * 1.4 * speed` radians).
    pub walk_position: f32,
    /// `walkAnimationSpeed`: how fast it walks, 0 standing to about 1.
    pub walk_speed: f32,
    /// Ticks left of the red hurt flash (10 when hit).
    pub hurt_time: i32,
    /// Ticks since it died, 0 while alive (vanilla tips the body over 20).
    pub death_time: i32,
    pub on_ground: bool,
}

impl<'a> NativeMobRender<'a> {
    /// The render state of `m` at `partial` (`LivingEntityRenderer.extractRenderState`).
    pub(super) fn of(m: &'a Mob, partial: f32) -> Self {
        let body = anim::body_rot(m, partial);
        // The walk cycle, head turn and age as the vanilla models read them:
        // no walking while riding or dead, a baby's legs three times as far
        // (`WalkAnimationState.position(partial)`, `positionScale`).
        let pose = anim::state(m, partial);
        Self {
            id: m.id,
            kind: &m.kind.id,
            custom_name: m.custom_name.as_deref(),
            baby: m.is_baby(),
            partial,
            age: pose.age,
            body_yaw: body,
            head_yaw: pose.y_rot,
            pitch: pose.x_rot,
            walk_position: pose.pos,
            walk_speed: pose.speed,
            hurt_time: m.hurt_time,
            death_time: m.death_time,
            on_ground: m.on_ground,
        }
    }
}

/// Which player a [`NativeMobRender`] is, for `native_player_submit` and
/// `native_mob_layer`.
// Native mods read it; nothing in Horizon does.
#[derive(Clone, Copy, Debug)]
pub struct NativePlayerRender<'a> {
    /// The player's profile name.
    pub name: &'a str,
    /// Whether it is the player of this window, drawn in third person.
    pub local: bool,
}

/// Whose name tag `native_name_tag` restyles.
// Native mods read it; nothing in Horizon does.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct NativeNameTag<'a> {
    /// The player's profile name.
    pub name: &'a str,
    /// Their UUID, where the game knows it.
    pub uuid: Option<u128>,
    /// Whether it is this window's own player (Horizon's own name tag extra).
    pub local: bool,
}

/// A mod's cape for a player (`NativePlayerLook::cape`): part `rect` (`[x, y, w, h]` pixels, the
/// game's 64x32 cape layout) of sprite `sprite`, as a cape texture is read. A run-time picture
/// is painted into the entity canvas (`native_entity_canvas_upload`, sprite
/// `MOD_ENTITY_CANVAS_SPRITE`).
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativeCape {
    pub sprite: &'static str,
    pub rect: [f32; 4],
}

/// One player's look this frame, as `native_player_look` reads and changes it before the player
/// is drawn (`AvatarRenderer.extractRenderState`'s end): who it is and what they wear, then what
/// the mod draws differently.
// Native mods read and set it; nothing in Horizon reads all of it.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NativePlayerLook<'a> {
    /// The player's profile name.
    pub name: &'a str,
    /// Their UUID, where the game knows it (a server's players, the player's own session).
    pub uuid: Option<u128>,
    /// Whether it is this window's own player (third person, the inventory's picture).
    pub local: bool,
    /// The feet in the world, at the frame's partial tick.
    pub position: [f64; 3],
    /// From the camera, squared (blocks).
    pub distance_sq: f64,
    pub crouching: bool,
    pub fall_flying: bool,
    /// Invisible (the effect, or the server's flag): only its armour and held items draw.
    pub invisible: bool,
    /// The item worn on the head and on the chest (`minecraft:diamond_helmet`).
    pub head_item: Option<&'a str>,
    pub chest_item: Option<&'a str>,
    /// The chest item draws wings (an elytra: `WingsLayer`, and no cape), or armour on the body
    /// (the cape moves out over it).
    pub chest_wings: bool,
    pub chest_armor: bool,
    /// Whether the player's own cape shows (they have one, and aren't invisible).
    pub has_cape: bool,
    /// The cape's flap, lean and lean2 in degrees (`AvatarRenderState.capeFlap`...): it turns back
    /// `6 + lean / 2 + flap` degrees.
    pub cape_angles: [f32; 3],
    /// Set: a cape of the mod's in place of the player's own (or none), its elytra matching.
    pub cape: Option<NativeCape>,
    /// Set: no cape at all (one would go through the mod's geometry).
    pub hide_cape: bool,
    /// Set: a pose over the model's (an emote), its layers following.
    pub pose: horizon_client::mobs::anim::PoseBlend,
    /// Set: `native_mob_layer` gets the posed parts for `BlockEntityDraw::attach`.
    pub parts: bool,
}

/// What a mod changed of the player being drawn now, for the layers drawn after
/// `native_player_look`.
#[derive(Clone, Copy, Debug)]
pub(super) struct ActiveLook {
    mob: u32,
    cape: Option<NativeCape>,
    hide_cape: bool,
    parts: bool,
}

thread_local! {
    /// The player being drawn's look; the window's thread draws one entity at a time.
    static ACTIVE: std::cell::Cell<Option<ActiveLook>> = const { std::cell::Cell::new(None) };
}

/// Holds a player's look while they're drawn: dropped, the next entity draws as the game's.
pub(super) struct LookGuard {
    blend: horizon_client::mobs::anim::PoseBlend,
}

impl Drop for LookGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| a.set(None));
        horizon_client::mobs::anim::set_player_blend(self.blend);
    }
}

/// The look a mod gave mob `id`, while it's drawn.
pub(super) fn active_look(id: u32) -> Option<ActiveLook> {
    ACTIVE.with(std::cell::Cell::get).filter(|a| a.mob == id)
}

impl ActiveLook {
    /// The mod's cape (`Some(None)`: none at all), or None to draw the player's own.
    pub(super) fn cape(&self) -> Option<Option<NativeCape>> {
        if self.hide_cape {
            Some(None)
        } else {
            self.cape.map(Some)
        }
    }
}

/// The pose a mod's mob model starts from: at the feet, `at` (relative to the
/// camera), turned with the body so +z is the way it faces, +x its left and +y
/// up, in blocks.
pub(super) fn mob_pose(at: DVec3, body_yaw: f32) -> Mat4 {
    Mat4::from_translation(at.as_vec3()) * Mat4::from_rotation_y((-body_yaw).to_radians())
}

impl Game<'_> {
    /// `native_mob_submit` for `mob`, lit by `light` (packed as the entity
    /// pass packs it): true when a mod drew it, and its vanilla model, armour
    /// and held items are left out. The name tag, shadow, fire and a glowing
    /// outline still draw as for any mob.
    pub(super) fn mod_mob_geometry(
        &self,
        mob: &Mob,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) -> bool {
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            let mut drawn = false;
            self.native_mob_submit(render, draw, &mut drawn);
            drawn
        })
    }
    /// `native_player_submit` for the player `mob` (`who` says which): true
    /// when a mod drew it and the vanilla model, armour, held items, cape and
    /// other layers are left out.
    pub(super) fn mod_player_geometry(
        &self,
        mob: &Mob,
        who: NativePlayerRender<'_>,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) -> bool {
        // Posing a player for the mods costs, each frame: only when one listens.
        let mut listened = false;
        self.native_player_render_listened(&mut listened);
        if !listened {
            return false;
        }
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            let mut drawn = false;
            self.native_player_submit(render, &who, draw, &mut drawn);
            drawn
        })
    }
    /// `native_mob_layer` for `mob` (`who` set for a player): what a mod adds
    /// is kept, drawn after the vanilla model and its layers.
    pub(super) fn mod_mob_layer_geometry(
        &self,
        mob: &Mob,
        who: Option<NativePlayerRender<'_>>,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) {
        // Every mob in view each frame: only when a mod listens.
        let mut listened = false;
        self.native_player_render_listened(&mut listened);
        if !listened {
            return;
        }
        // The posed parts, only for a player a mod asked them for (`NativePlayerLook::parts`).
        let parts = active_look(mob.id).filter(|l| l.parts).map(|_| {
            horizon_client::mobs::render::humanoid_parts(mob, partial, camera, self.res.mob_data())
        });
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            if let Some(parts) = parts {
                draw.set_parts(parts);
            }
            self.native_mob_layer(render, who.as_ref(), draw);
            true
        });
    }
    /// `native_player_look` for the player `mob` (`who` says which; `head` and `chest` the items
    /// they wear there), before they're drawn: what the mod changed holds until the guard drops,
    /// at the end of this player's draw. None when no mod listens (`native_player_render_listened`).
    pub(super) fn begin_player_look(
        &self,
        mob: &Mob,
        who: NativePlayerRender<'_>,
        partial: f32,
        camera: DVec3,
        head: Option<&str>,
        chest: Option<&str>,
    ) -> Option<LookGuard> {
        let mut listened = false;
        self.native_player_render_listened(&mut listened);
        if !listened {
            return None;
        }
        let render = NativeMobRender::of(mob, partial);
        let feet = mob.previous.lerp(mob.position, f64::from(partial));
        let feet = DVec3::new(feet.x, feet.y, feet.z);
        let layers = chest
            .and_then(|item| self.res.items().get(item))
            .and_then(|i| i.equip_asset.as_ref())
            .and_then(|asset| self.armor_stand_assets.equipment.get(asset));
        let uuid = if who.local {
            self.own_profile_uuid()
        } else {
            self.remote_player_uuid(mob.id)
        };
        let mut look = NativePlayerLook {
            name: who.name,
            uuid,
            local: who.local,
            position: feet.to_array(),
            distance_sq: feet.distance_squared(camera),
            crouching: mob.extra.crouching,
            fall_flying: mob.extra.fall_flying,
            invisible: mob.invisible || horizon_client::mobs::trader::is_invisible(mob),
            head_item: head,
            chest_item: chest,
            chest_wings: layers.is_some_and(|k| k.contains_key("wings")),
            chest_armor: layers.is_some_and(|k| k.contains_key("humanoid")),
            has_cape: mob.cape.is_some() && !horizon_client::mobs::trader::is_invisible(mob),
            cape_angles: horizon_client::mobs::render::cape_angles(mob, partial),
            cape: None,
            hide_cape: false,
            pose: horizon_client::mobs::anim::PoseBlend::default(),
            parts: false,
        };
        self.native_player_look(&render, &mut look);
        let blend = horizon_client::mobs::anim::set_player_blend(look.pose);
        ACTIVE.with(|a| {
            a.set(Some(ActiveLook {
                mob: mob.id,
                cape: look.cape,
                hide_cape: look.hide_cape,
                parts: look.parts,
            }))
        });
        Some(LookGuard { blend })
    }
    /// Gives `hook` the frame of `mob` and a draw posed at its feet; what it
    /// added to `out` stays when it answers true.
    fn mod_entity_draw(
        &self,
        mob: &Mob,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
        hook: impl FnOnce(&NativeMobRender<'_>, &mut BlockEntityDraw) -> bool,
    ) -> bool {
        let render = NativeMobRender::of(mob, partial);
        let a = f64::from(partial);
        let feet = mob.previous.lerp(mob.position, a);
        let feet = DVec3::new(feet.x, feet.y, feet.z);
        let start = out.len();
        let mut draw = BlockEntityDraw::posed(
            partial,
            self.world.game_time,
            light,
            [feet.x, feet.y, feet.z].map(|c| c.floor() as i32),
            mob_pose(feet - camera, render.body_yaw),
            diffuse_lights(self.dim == Dim::Nether),
            std::mem::take(out),
        );
        let keep = hook(&render, &mut draw);
        *out = draw.into_vertices();
        if !keep {
            out.truncate(start);
        }
        keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn the_pose_faces_where_the_body_faces() {
        // Entity yaw 0 faces south (+z), 90 west (-x), 180 north, 270 east.
        for (yaw, ahead, left) in [
            (0.0, Vec3::Z, Vec3::X),
            (90.0, Vec3::NEG_X, Vec3::Z),
            (180.0, Vec3::NEG_Z, Vec3::NEG_X),
            (270.0, Vec3::X, Vec3::NEG_Z),
        ] {
            let pose = mob_pose(DVec3::ZERO, yaw);
            assert!(close(pose.transform_vector3(Vec3::Z), ahead), "{yaw}");
            assert!(close(pose.transform_vector3(Vec3::X), left), "{yaw}");
            assert!(close(pose.transform_vector3(Vec3::Y), Vec3::Y), "{yaw}");
        }
    }

    #[test]
    fn the_pose_starts_at_the_feet() {
        let at = DVec3::new(3.0, -1.5, 0.25);
        let pose = mob_pose(at, 37.0);
        assert!(close(pose.transform_point3(Vec3::ZERO), at.as_vec3()));
    }
}
