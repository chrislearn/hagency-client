#import <AppKit/AppKit.h>
#import <Sparkle/Sparkle.h>
#include "sparkle_bridge.h"

static char *json_string(id value) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:NULL];
    return data ? strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String) : NULL;
}

char *ha_sparkle_prototype_config(void) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on the main thread");
    NSBundle *bundle = NSBundle.mainBundle;
    if (![bundle.bundleIdentifier isEqualToString:@"org.hagency.desktop.sparkle-prototype"]) return NULL;
    NSDictionary *info = bundle.infoDictionary;
    return json_string(@{@"root": info[@"HagencySparklePrototypeRoot"] ?: @"",
                         @"version": info[@"CFBundleVersion"] ?: @"",
                         @"bundle": bundle.bundlePath,
                         @"feed": info[@"SUFeedURL"] ?: @""});
}

@interface HASparkleBridge : NSObject <SPUUserDriver, SPUUpdaterDelegate>
@property(nonatomic, strong) SPUUpdater *updater;
@property(nonatomic, strong) NSMutableArray<NSDictionary *> *events;
@property(nonatomic, copy) void (^foundReply)(SPUUserUpdateChoice);
@property(nonatomic, copy) void (^readyReply)(SPUUserUpdateChoice);
@property(nonatomic, copy) void (^cancellation)(void);
@property(nonatomic, copy) void (^retryTermination)(void);
@property(nonatomic) uint64_t cycle;
@property(nonatomic) uint64_t received;
@property(nonatomic) uint64_t expected;
- (void)emit:(NSString *)kind values:(NSDictionary *)values;
@end

@implementation HASparkleBridge
- (void)emit:(NSString *)kind values:(NSDictionary *)values {
    NSCAssert([NSThread isMainThread], @"Sparkle callback left main thread");
    NSMutableDictionary *event = [@{@"kind": kind, @"cycle": @(self.cycle)} mutableCopy];
    [event addEntriesFromDictionary:values];
    [self.events addObject:event];
}
- (void)showUpdatePermissionRequest:(SPUUpdatePermissionRequest *)request reply:(void (^)(SUUpdatePermissionResponse *))reply {
    // The harness deliberately disables automatic checking/downloading in Info.plist.
    reply([[SUUpdatePermissionResponse alloc] initWithAutomaticUpdateChecks:NO sendSystemProfile:NO]);
}
- (void)showUserInitiatedUpdateCheckWithCancellation:(void (^)(void))cancellation {
    self.cancellation = cancellation;
    [self emit:@"checking" values:@{}];
}
- (void)showUpdateFoundWithAppcastItem:(SUAppcastItem *)item state:(SPUUserUpdateState *)state reply:(void (^)(SPUUserUpdateChoice))reply {
    self.cancellation = nil;
    if (item.informationOnlyUpdate || state.stage != SPUUserUpdateStageNotDownloaded) {
        reply(state.stage == SPUUserUpdateStageInstalling ? SPUUserUpdateChoiceSkip : SPUUserUpdateChoiceDismiss);
        [self emit:@"error" values:@{@"message": @"Prototype refuses information-only or previously staged updates"}];
        return;
    }
    self.foundReply = reply;
    [self emit:@"available" values:@{@"version": item.displayVersionString ?: @"", @"build": item.versionString ?: @""}];
}
- (void)showUpdateReleaseNotesWithDownloadData:(SPUDownloadData *)data {}
- (void)showUpdateReleaseNotesFailedToDownloadWithError:(NSError *)error {
    [self emit:@"notes_error" values:@{@"message": error.localizedDescription}];
}
- (void)showUpdateNotFoundWithError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"current" values:@{}];
    acknowledgement();
}
- (void)showUpdaterError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"error" values:@{@"message": error.description}];
    acknowledgement();
}
- (void)showDownloadInitiatedWithCancellation:(void (^)(void))cancellation {
    self.received = 0;
    self.expected = 0;
    self.cancellation = cancellation;
    [self emit:@"downloading" values:@{}];
}
- (void)showDownloadDidReceiveExpectedContentLength:(uint64_t)length { self.expected = length; }
- (void)showDownloadDidReceiveDataOfLength:(uint64_t)length {
    self.received += length;
    [self emit:@"progress" values:@{@"received": @(self.received), @"expected": @(self.expected)}];
}
- (void)showDownloadDidStartExtractingUpdate {
    self.cancellation = nil;
    [self emit:@"extracting" values:@{}];
}
- (void)showExtractionReceivedProgress:(double)progress {}
- (void)showReadyToInstallAndRelaunch:(void (^)(SPUUserUpdateChoice))reply {
    self.readyReply = reply;
    [self emit:@"ready" values:@{}];
}
- (void)showInstallingUpdateWithApplicationTerminated:(BOOL)terminated retryTerminatingApplication:(void (^)(void))retry {
    self.retryTermination = terminated ? nil : retry;
    [self emit:@"installing" values:@{@"terminated": @(terminated)}];
}
- (void)showUpdateInstalledAndRelaunched:(BOOL)relaunched acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"installed" values:@{@"relaunched": @(relaunched)}];
    acknowledgement();
}
- (void)dismissUpdateInstallation {
    self.foundReply = nil;
    self.readyReply = nil;
    self.cancellation = nil;
    self.retryTermination = nil;
    [self emit:@"dismissed" values:@{}];
}
@end

void *ha_sparkle_create(void) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    if (![NSBundle.mainBundle.bundleIdentifier isEqualToString:@"org.hagency.desktop.sparkle-prototype"]) return NULL;
    HASparkleBridge *bridge = [HASparkleBridge new];
    bridge.events = [NSMutableArray new];
    bridge.updater = [[SPUUpdater alloc] initWithHostBundle:NSBundle.mainBundle applicationBundle:NSBundle.mainBundle userDriver:bridge delegate:bridge];
    NSError *error = nil;
    if (![bridge.updater startUpdater:&error]) [bridge emit:@"error" values:@{@"message": error.description ?: @"Sparkle startup failed"}];
    return (__bridge_retained void *)bridge;
}

char *ha_sparkle_next_event(void *handle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    if (bridge.events.count == 0) return NULL;
    NSDictionary *event = bridge.events.firstObject;
    char *result = json_string(event);
    [bridge.events removeObjectAtIndex:0];
    return result;
}

bool ha_sparkle_action(void *handle, uint32_t action, uint64_t cycle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    if (action == 1 && bridge.updater.canCheckForUpdates && !bridge.updater.sessionInProgress) {
        bridge.cycle += 1;
        [bridge.updater checkForUpdates];
        return true;
    }
    if (bridge.cycle != cycle) return false;
    if (action == 6) { // Isolated harness: exercise AppKit's real QuitRequested path.
        dispatch_async(dispatch_get_main_queue(), ^{ [NSApp terminate:nil]; });
        return true;
    }
    if (action == 2 && bridge.foundReply) {
        void (^reply)(SPUUserUpdateChoice) = bridge.foundReply;
        bridge.foundReply = nil;
        reply(SPUUserUpdateChoiceInstall);
        return true;
    }
    if (action == 3 && bridge.readyReply) {
        void (^reply)(SPUUserUpdateChoice) = bridge.readyReply;
        bridge.readyReply = nil;
        reply(SPUUserUpdateChoiceInstall);
        return true;
    }
    if (action == 4) {
        if (bridge.readyReply) {
            void (^reply)(SPUUserUpdateChoice) = bridge.readyReply;
            bridge.readyReply = nil;
            reply(SPUUserUpdateChoiceSkip); // Dismiss would still install at exit.
        } else if (bridge.foundReply) {
            void (^reply)(SPUUserUpdateChoice) = bridge.foundReply;
            bridge.foundReply = nil;
            reply(SPUUserUpdateChoiceDismiss);
        } else if (bridge.cancellation) {
            void (^cancel)(void) = bridge.cancellation;
            bridge.cancellation = nil;
            cancel();
        } else return false; // Extraction is not cancellable until ready/error.
        return true;
    }
    if (action == 5 && bridge.retryTermination) {
        void (^retry)(void) = bridge.retryTermination;
        bridge.retryTermination = nil;
        retry();
        return true;
    }
    return false;
}
void ha_sparkle_free_string(char *value) { free(value); }
void ha_sparkle_destroy(void *handle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge_transfer HASparkleBridge *)handle;
    // The driver and updater own one another while active. Break this cycle only
    // when the Rust owner is done, never to pretend that an installer was canceled.
    bridge.updater = nil;
}
