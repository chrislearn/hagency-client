// Read actual native screenshot pixels when Makepad's widget snapshot does not
// expose dynamic StackNavigation children. No network service is used.
import Foundation
import Vision
import AppKit

guard CommandLine.arguments.count == 2 else { exit(2) }
let url = URL(fileURLWithPath: CommandLine.arguments[1])
let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en-US"]
request.automaticallyDetectsLanguage = true
request.usesLanguageCorrection = false
try VNImageRequestHandler(url: url).perform([request])
let result = (request.results ?? []).compactMap { observation -> [String: Any]? in
    guard let text = observation.topCandidates(1).first else { return nil }
    let rect = observation.boundingBox
    return ["text": text.string, "confidence": text.confidence,
            "box": [rect.minX, 1 - rect.maxY, rect.width, rect.height]]
}
let data = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
FileHandle.standardOutput.write(data)
