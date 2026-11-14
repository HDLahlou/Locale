Locale

Product Requirement Document

**Mission Statement:**  
Create an application that allows for temporary peer to peer data connection and file hosting, allowing for file transfer, communication, forums, and even location.

**General Description:**  
Turn cellphones into servers. Circumvent the cloud for hosting temporary forums and files. An app to temporarily turn your phone into a server. Allowing for open sharing between Apple and Android or computer. Allows for fidelity for media quality, as most apps compress for space efficiency since it is hosted on their cloud servers. Since we are targeting temporary file storage and use local data stores, we can avoid file compression and share high quality content easily (as messaging services will default to lowering quality)

**What is it similar to:**  
Essentially a locally hosted temporary discord app, where the servers are event pages.

**What Problems does it solve:**

- Prevents Data Quality loss  
- Data privacy from the cloud  
- Sharing between different devices easily  
- Not everything needs to be permanent and live on the internet  
- Solution to stay connected without wifi/service  
- Solutioning for local events

**What the Server/Service offers:**

- User profiles  
  - Temp profiles for casual events  
  - Permanent profiles for more serious events (for security reasons)  
    - Friend/Connections?  
  - Linking to other app profiles for convenience  
    - Spotify, Google/Drive, Venmo, Instagram  
- Store Profile  
  - An entity managed by a user (like a listing or a page)  
- Event Listings  
  - Searchable by keywords  
    - shop, event, meet up, etc.  
  - Secure Events:  
    - Invite only  
    - Password events  
    - Request \+ Approval  
  - Proximity only  
  - Same data networks  
  - Bluetooth connectivity only  
- Event Pages:  
  - Invite links  
  - Event description  
  - Times  
  - External Links  
  - RSVP list  
- Session Data  
  - Photos  
  - Video  
    - Actual file sharing  
    - Streaming of screen/video content? (Screenshare but one to many vs one to one on a tv screen)  
    - Cowatching experience for things like youtube?  
  - Music/Audio files  
    - Would this be like a calling feature  
    - Actively Streaming to eachother?  
      - Would it require the actual audio file? Unless we stream purely one phones audio? Or a certain channel of audio?  
      - Can we differentiate between different audios? Notification vs music/media  
    - A link to the Jam?  
      - Incorporate other apps natively?  
      - Start with Spotify API  
  - Chats  
    - General Chat  
    - Private Chats  
      - Called lo-key  
    - Other channels/channel creation  
    - Host only channels for announcements  
  - Feed  
    - Posts/Threads  
      - Comment feature  
  - Polls/Votes  
  - Location Data  
    - Finding eachother  
      - Has to be opted into  
      - Can use GPS services or local pinging  
      - Point in direction to find person  
      - Great when hiking or at a festival with poor reception  
    - Finding stands/vendors  
    - Local find my friends  
  - Games  
    - Gambling?  
      - Accrue points to add wagers or bets:  
        - Ex. if X thing will happen, if X person will win, if X person will do Y thing  
    - Clues and instructions  
      - Abstracts the rules/handling to a third party  
    - Can suggest or vote on certain games  
      - Board games vs rage cage vs pictionary  
        - For things that require prompts it can provide that  
    - Basic games  
      - random word generators  
        - could use ai based on categories provided to choose a word/dataset  
      - number generators  
      - name generator  
        - name pairs generator (Sarah \+ alex)  
      - score tracker  
      - point system  
        - use points earned over time to pay for submitting certain words or names or requests or party wide text blasts  
      - Jack in the box inspo  
  - Ledgers, like splitwise  
    - Image upload or manual input  
    - Auto split  
    - Auto parse/itemize  
    - People able to click on item to auto split or claim costs  
      - Calculates tip and tax  
    - Can incorporate venmo  
    - For tracking purposes can email or text final sums/itemized lists to ensure payments are covered  
    - Could allow for transfer to other apps like splitwise, or auto charging on venmo  
- Session Data Qualities:  
  - Tags  
    - Context  
    - NSFW  
  - Data Restrictions  
    - Text only or image only  
- Temporary Data Retention  
  - Can be set for general session  
    - Example: deletes after 24 hrs  
  - Can be set per object?  
    - Example: A quick notice reminder that deletes in 60 mins  
- Security:  
  - Only verified or permanent profiles can upload  
    - This allows to prevent malicious randoms  
      - Safety for viruses  
      - Safety for minors  
      - Allows for traceability  
  - Temporary accounts can still view and download  
  - Preventing school wifi or accounts?  
  - Should there be posting limits and timeout windows to prevent spam?  
    - Could be default setting  
  - File scanning to prevent injection and viruses  
  - Create a log that tracks lowlevel activities by text

**Use Cases:**

- Want to focus on local events and in person interaction  
- Could later expand to be not limited to local events (streams)  
- Productivity:  
  - Create a hub/open stream  
    - Allows for easy file share between work groups  
  - All devices can grab or download files  
  - Docs and files? Like google doc?  
  - Shared todo lists  
  - Notes board  
- Formal Sessions vs   
  - Parties/Event Hosting  
  - Festivals where connection is unreliable  
  - Markets/Fairs  
    - Can post products, pricing for street vendors  
  - Hiking  
    - Offline Scenario  
  - Academic classroom settings  
    - Needs to be limited to edu accounts  
- Local Event Finding:  
  - Interest/Hobby groups  
  - Dancing  
  - Trivia nights  
- Impromptu Sessions  
  - General Hang out/ Night out

**Question:**

- Do we want all hosting to be dependent on the HOST device, or have limited hosting capabilities on cloud/app server, to allow for creating an event and sharing it out, essentially like a place holder. All actual activity and event sensitive data will be stored by host  
  - Do we want to have upgrades available? Does this defeat the purpose  
- Only allow content to be posted by people within the proximity, to keep it to voices at the event?  
- Have phones ping eachother? This would help with offline user recognition and location services  
- Do we want to limit communication capabilities?  
  - Sending text or email blasts

**Data Problem:**   
When storing data on multiple phones to increase peer to peer data capacity, how do we distribute the load, ensure data integrity and ensure uptime (As in, where is the actual data, how do we keep track of it, and what happens if a host/phone turns off intentionally or by accident)

**Main Technical Design Questions:**

- For access to device file systems and management, needs to be an app with native access  
- Data:  
  - How to ensure data integrity and fault tolerance?  
  - Should we only allow file access from within the app directory  
    - Prevents complication of searching for outside files  
    - Question: What does this mean when a user is the original sharer of an image from their device? This approach would duplicate the data or move their file which are both unacceptable  
      - Answer: If a host is the original creator/uploader of an image, keep a path to that file for grabbing purposes   
        - If possible, prevent the moving or deletion of that file by having the app access it, similar to when an app is open on a computer  
      - Answer: Only store files in app directory and use that approach for backup data when downloading.  
        - Either way, the solution is to have a path to the actual file, if inaccessible, invalidate that entry and look for valid backup from other hosts/users. If none exist, prompt the user if they can reupload the image  
          - How do we let the user know what image to do?   
            - A cached compressed preview of the image can be shown for reference. The original file wants to be shared for its quality so this would not be a suitable replacement  
            - This means that there will have to be a main table/store that has this data, could be a part of the main ID store pointer table  
- What is the best way to store a thread and its comments?  
  - Look at how social media posts are stored

Example:

- Multiple Hosts w/ master host  
  - For node failsafe, have list of back up data, with backups being read only copies and host having only write access  
    - Should backups be written to after every access or on interval period  
- Data is stored on one node  
  - Should we inherently have a back up for the data? Store on two different phones/nodes  
  - Once content is viewed or downloaded, that new person can now be a back up node if that data store goes down  
    - Can keep a linked list of users or potential node back ups  
      - Question: how will we know what file we want from that user  
      - Answer: Keep file in app folder store, if not there then dont bother with it  
- Lock on ID matching table instead of data store so that way it is quick access and blocking  
- Allow users to select how much data they want to allocate for app storage

How do we want to handle displaying content to users?

Options:   
Ping host phone for content to display

- Pros:  
  - Central control  
  - One source of truth  
- Cons:  
  - Constant load on host  
    - Whole point is to disperse the load  
  - One point of failure

Users have local table to render and display content

- Pros:  
  - Multiple sources of truth  
  - Not just one point of failure  
  - Less load on main node  
  - Fault tolerance  
- Cons:  
  - Overcomplication

What is stored on Host Devices:

- Large Data:  
  - Images/Videos, etc  
- Dynamic Data:  
  - Polls and threads  
  - Constantly changing, want a source of truth  
- Secure data:  
  - Ledgers  
  - Sensitive data flagged by user  
    - Passwords, Event details, etc  
- Hosting Data Streams  
- Local Events (Party/Proximity)

What is stored on Users:

- Private messages

What is stored by Service Provider (app itself):

- User Profiles and relevant user data  
- Larger Events (Fairs and vendors)

**Technical Flow:**

TODO:

* Sync Protocal  
* Flagged Content Queue/Processor  
* Add max queue attempt to Stuck queue  
  * Or increase backoff timer  
* Replace time-in-state with a heartbeat or last-progress timestamp. A slow precheck on a large file will otherwise be reaped at 2 minutes while the client is still working.  
* Add TTL to locks  
* Seperate sub routines to its own page  
* "Terminal states can still be updated" is a contradiction. Make the payload immutable and the metadata mutable.  
* State Transitions table  
* **Data Stream.**  
  * The failover loop back to the top has no iteration cap or exhausted-list exit.  
  * "Close/strong enough" and "enough active nodes" need thresholds.  
  * Use chunked, resumable uploads, and define the large-file cutoff and when Bluetooth defers to the queue.  
* **Index sync.**  
* Add gap detection with a full-resync fallback, idempotent apply, and tombstones for invalidated content.  
* Define what "validate that content matches" compares.  
* One change has three delivery paths (live push, webhook, enqueued processing). Define precedence and dedupe.  
* Connection Validation enqueues Index Update Processing, but the subscriber starts at "client received notification," which nothing sends in that case.  
* **Hygiene**  
* **Observability.** Add a correlation ID per submission, metrics (queue depth, stuck count, state age), alerts, and an audit log of denials and reviews.  
* **Cleanup and retention.** Handle partial blobs on hosts after failures, and set retention for FAILED and QUEUED entries. Index the sweeper's (state, updated\_at) query.  
* **Docs and diagram.**  
  * Add a glossary (main node, host, server; index table, ID table, ID directory) and a topology plus happy-path sequence.  
  * The note on the right of Submission flow is still clipped, and the "Hosts?" label collides with its diamond.  
  * The main node as both control plane and fallback host is a single point of failure.  
* Outbox pickup  
* Authorize data stream with upload token  
* If session outlasts token, reauthorize

Allow users to join session through nfc tapping

**Item States:**

- Processing states:  
  - New \-\> In Progress  
  - If item is in this state for an extended period they are considered stuck  
  - NOTE: Large uploads might still be uploading and are not stuck  
- Enqueued State:  
  - Queued  
    - Was previously stuck/not properly uploaded  
    - Could also be stuck  
- Terminal States:  
  - Competed/Uploaded \-\> Failed  
  - Updates can still be potentially performed to these entries, to either invalidate or update entry to new end point

**Checking for Stuck Content:**

- Triggers  
  - Routine check every X mins  
  - Checking index table for entries stuck in initial state (has not been uploaded) or queued state  
    - If it has not changed in the past X minutes submit to queue  
    - Keep in mind large data sets might still be uploading  
      - Validate this before enqueuing

**Draining the Queue:**

- Pick up message in queue  
- Check if still valid  
  - Drop if not, notify source if possible  
- If valid, check if message has already been processed  
  - Drop if not to prevent duplicates  
- If new message, reauthenticate if user access is still allowed  
  - If not allowed return access error to user  
- Commence Submission flow  
  - Should be done in separate thread

**Retry Mechanism:**

- Max 3 Retries  
- 3 Second Backoff  
- Timeout attempts after 3 seconds of no response  
  - No response condition since uploads could be slow

**User Content Rendering:**

- To prevent constant polling of host, render content based on local id table  
- Upon opening, user will be given an index table  
- Host will push updates to current open sockets or users

**User Prevalidation:**

1. Is signed in/ valid session  
2. Is an established account  
   1. Has access/necessary roles, both as a general user and for the event/channel itself  
      1. If not, prevent option from being selected in the first place  
      2. Back up authentication for user action as second layer of validation  
   2. Can only upload if permanent account  
   3. Can read and download if guest account  
3. Should rate limit check be done here?

**Connection Prevalidation:**

1. Is able to contact main host node (Ping/Handshake)  
2. Query current ID table to ensure proper access  
   1. Check versioning  
   2. If different than local pull update

**Content Precheck Validation:**

1. Verify acceptable file type  
2. Verify type through file content/ magic bytes  
3. Make sure file size limit isn't exceeded  
4. *Duplicate Idempotency Check*  
5. Scan object for malicious content (viruses)  
   1. Text: Check for any injections  
   2. Images:   
6. Strip any metadata

**Duplicate Idempotency Check:**

- With provided context, check to see if any data in index has duplicate content  
- Content Hash?  
- Flag as duplicate and resolve manually by user  
- Return duplicate and likely original  
- Exclude the submission's own ID.  
- 

**Suitable Host Determination: (Similar to load balancing)**

1. Grab pool of current Hosts  
2. Confirm Host availability  
   1. Ping hosts  
3. Analyze traffic trends and host traits  
   1. Current traffic  
      1. If one Host is getting a lot of traffic and another is not, prioritize lesser used node  
   2. Recency bias  
      1. If an item was just uploaded to node, deprioritize that node as new traffic could be ramping up  
         1. Once time window has passed, reset this bias  
   3. Storage Space  
      1. If node is reaching data limit, deprioritize uploading further content   
   4. Consider host specialization?  
      1. Text vs image nodes  
   5. Consider host proximity?  
      1. Closer/easier data transfer?  
4. Return ranked list, to allow for resilience if top selection host drops

Question: How to optimize? Would creating clusters or specialized hosts/nodes increase efficiency? A node for hosting text threads would allow for fast forwarding without the cost of looking up directory for each individual piece.

**Data Stream:**

- Wifi:  
  - Does the session allow for wifi based transfer?  
    - Assume default enabled  
  - Are the users on the same network?  
  - Does the session allow for out of network data transfer?  
- Bluetooth:  
  - Uses bluetooth peer to peer connection  
  - If strong enough connection, can directly communicate with client and server to upload/download  
  - If too far apart, bounce off of other devices and nodes to send data  
- If no valid options are available, enqueue for later upload  
- NOTE: Should uploads or data streams be performed in separate thread? Holding the initial setup and validation thread for data submission makes everything inline, might be beneficial to make a background process done in parallel once validations are completed  
  - Should be done for large file sizes  
  - Will need disjointed async flow for upload resolution/ failure flow

**Index Table:**  
Table that contains context for:

- Where objects are stored (Pointer to HOST)  
  - Allows user to find host with desired data  
- Backups for data  
  - Linked list of others who have downloaded the content  
- History Column  
  - Who has viewed and/or downloaded the file  
- Content Previews  
- Original uploader/authors  
- Content Type  
- Content Tags

Read Replicas to ensure data integrity  
Lock on Writes to ensure integrity  
On update, sends read only copy for other auxiliary nodes to store

- Q: What happens when a user requests info while the table is updating?  
  - A: User will be polling/updating for new content repeatedly

**Downstream Index Update Notification:**  
Best approach is to send updates on:

- Request   
  - User accesses/refreshes app  
  - Fetch latest content  
- Event submission  
  - On event submission that would update/change the ID table and impact content availability, send updates/webhooks to other nodes or any current active connections  
    - Take current pool of sockets/connections and push update/webhook  
    - Concurrent users get live update  
    - Other users will get event webhook for background updating  
  - What happens for non active users?  
    - Q: Should we queue event webhooks or mark their account as out of sync?  
    - A: Not needed, as the user will poll for up to date content on opening of app  
      

**Index Update Processing:**

- Q: To optimize, we would want to only push what data is necessary, could be based off of version difference

**Upload Large Item Flow:**

1. *User Prevalidation*  
2. *Object Selection*  
3. Object Type Determination  
   1. Image, Video, etc  
4. *Content Precheck Validation*  
   1. If duplicate, require manual review  
5. *Connection Prevalidation to main Node*  
6. **Conditional**: IF there is more than one host:  
   1. *Suitable Host Determination* for object  
7. **Optional**: Generate/Select related tags  
8. Determine if upload should be done inline or async in separate thread  
   1. Only upload async if large file size  
9. Reperform validations on Server Side  
10. Insert tracking entry into Index table  
11. Encrypt Item  
12. Upload Item to determined host  
    1. Perform **Data Stream**  
    2. Validate content integrity/checksum  
       1. Mismatch triggers retry  
       2. After max attempts, return error  
13. Update index table with completion state  
    1. Push update to tracking hosts/clients  
14. Return completion acknowledgment

**Retrieving Large Item Flow:**

1. Item Selected by User  
2. *User Prevalidation*  
3. *Connection Prevalidation*  
4. Item Validation  
   1. Ensures content has not been altered or replaced  
5. Find relevant entry in Index table  
6. Update Index table for history and potential back ups  
7. Download Item  
   1. **Data stream** \+ validation  
8. Decrypt Item

**Threads/Text Storage:**

- Open Socket  
- On event, commits to db

**Messaging:**

- Open Socket

**Item Fall Back:**

- If host goes down or item/data has been compromised  
- Check index table for backups  
- If valid, pop item from linked list and replace item pointer to that new node

**Host Node Initialization:**

- Check capabilities:  
  - Permissions enabled  
  - Check power/battery  
  - Test Connection  
  - Test Processing power (hits minimum)  
  - Check space availability (hits minimum)  
  - Proximity  
    - Is within range of network, other hosts or users

**Host Node Decommision:**

- 

Hosting Implementation Stages:

- Phase 1: 1 phone acts as data host  
- Phase 2: Multiple SET phones hosting  
- Phase 3: Multiple hosts with possibility of dropping and switching hosts

Functionality Stages:

- Phase 1:   
  - User Profiles  
  - Event Listings  
  - Image sharing  
  - Other static content  
- Phase 2:   
  - Private chat  
  - feed \+ thread  
  - Live content  
  - text chat  
- Phase 3: Video/Audio Streams

