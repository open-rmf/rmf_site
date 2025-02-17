# Undo Functionality

The undo functionality is handled by the `RevisionTracker` resource.

## Implementing Undo for your own plugin

Bevy is designed to be extended using `Plugin`s. The `RevisionTracker` builds on this
ideal.

## Design Considerations

There are several possible ways to implement undo functionality. The old Traffic Editor used
to build on Qt's QAction functionality. This relies heavily on C++'s object oriented nature.
While it is possible to use `dyn` and `Arc` in rust to try to replicate this method, it does not
play well with Bevy's event driven ECS nature. Rather we focus on creating a resource which generates unique ids.
Each unique ID corresponds to an action. It is up to individual plugin authors to handle the
buffer which stores the changes in state. It is recommended that one maintains a hashmap with the action id being the key and a custom struct that represents your change.

## Implementing your own Undo-able action

If your plugin simply changes a component, it is recommended that you use the `Change` event and associated tools.
The change component itself does a lot of the heavy lifting for the components and will automatically provide you with feedback. If you are doing more than just changing
a component then you probably need to read the rest of this section.

### Manually storing actions

The best reference for how to implement undo in your action is perhaps by reading the change plugin's source code. However,
for the sake of good design documentation, this section will try to explain how you can implement undo for your plugin.

When making a change to the world, the first thing you need to do is request a new revision id from the `RevisionTracker` resource. This revision ID is the unique
ID for the specific action. Your plugin should store it along with the required information to undo the change. When a user wants
to undo something the `UndoEvent` event is emitted. Your plugin should implement a listener for this event. The event itself will tell you which action is being undone.