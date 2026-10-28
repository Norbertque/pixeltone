# Pixeltone

Pixeltone is a bit-perfect music player for Windows and Android. A phone can control the
player on a PC, and friends on the same Wi-Fi can share its queue in a jam.

## Music

**Track**:
One audio file as the player knows it: in a library, in the queue or playing.
_Avoid_: Song

**Library**:
The tracks Pixeltone found in the music folders of the device it runs on.
_Avoid_: Collection

**Queue**:
The ordered list of tracks the player plays.
_Avoid_: Up next, playlist (a playlist is saved; the queue is what plays)

## Remote control

**Paired phone**:
A phone that was paired with a PC once and may control it from then on.
_Avoid_: Paired device, remote

**Hand-over**:
Moving playback of a phone's queue to the PC, which then plays the phone's tracks.
_Avoid_: Cast, transfer

**Phone track**:
A track that comes from a phone rather than from the PC's library.
_Avoid_: Remote track, upload

## Jam

**Jam**:
A temporary shared queue on a PC running Pixeltone, which guests join with its QR code or
link. It ends when the owner ends it or Pixeltone quits.
_Avoid_: Session, party, group session

**Owner**:
Whoever controls the PC during a jam: the person at the PC and every paired phone.
_Avoid_: Host, admin

**Guest**:
Someone who joined a jam with its QR code or link. A guest is never paired and loses access
when the jam ends.
_Avoid_: Participant, listener, user

**Guest permissions**:
What guests may do in a jam: add tracks, change the tracks they added, control playback.
_Avoid_: Rights, roles

**Removed guest**:
A guest the owner took out of a jam. They can come back into that jam only through a rejoin
request.
_Avoid_: Kicked, banned

**Rejoin request**:
A removed guest's request to come back into the same jam, which the owner allows or declines.
_Avoid_: Invite
