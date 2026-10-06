# rmf_site_format

File format definition and parsing for the [rmf_site_editor](https://github.com/open-rmf/rmf_site).

## Zones

Each level can store a `zones` map keyed by site entity ID. A zone has a name, an ordered list of anchor IDs on that level, and optional set memberships. The last anchor connects back to the first:

```json
{
  "zones": {
    "14": {
      "anchors": [10, 11, 12, 13],
      "name": "Cleaning area"
    }
  }
}
```

Existing site files load without changes. Levels with no zones omit the collection when saved. `Zone::convert` remaps anchor and set IDs and reports missing references.

Select a level and choose **Tool → Add Zone**. Draw as you would a floor: place at least three vertices, then click the first vertex to close the polygon. Escape finishes a polygon with enough vertices. Otherwise, it cancels the polygon and removes its temporary anchors.

Select a zone to rename it, move its anchors, or delete it. Concave polygons are supported. Boundaries that cross themselves or enclose no area are shown as outlines so you can fix them.

**View → Zones** shows or hides zones independently of floors. This also applies to newly drawn zones, so turn it on before drawing. Hidden zones are still saved, but the visibility setting itself is not stored in the site file.

Zones mark named areas on the map.

The [office map](../../assets/demo_maps/office.site.json) includes example Wi-Fi survey zones along the desk aisle, in the hardware room, and in a conference-room corner. The zones reuse existing anchors from the office layout. Select a zone to see its survey label.

## Zone sets

Open **Tabs → Zone Sets** to create, rename, or delete named sets. Select a zone and use the **Zone sets** checkboxes in **Inspect** to add it to one or more sets. The **Show** menu in **Zone Sets** filters the map to all zones, zones without a set, or a named set. New zones drawn while viewing a named set are added to that set. Deleting a set removes its memberships and keeps the zones.

Sets are shared across levels and saved in the site's `zone_sets` map. Each zone's optional `sets` list stores its set IDs. Set names and memberships survive saving and reopening; the display filter is an editor setting. **View → Zones** controls visibility together with the set filter.
