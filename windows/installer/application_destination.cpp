#include "policy.h"

#include <stdexcept>

namespace ame::installer {
namespace {

bool Contains(const std::filesystem::path& parent, std::filesystem::path child) {
  while (!child.empty()) {
    if (SamePath(parent, child)) {
      return true;
    }
    const auto next = child.parent_path();
    if (next == child) {
      break;
    }
    child = next;
  }
  return false;
}

}  // namespace

void ValidateApplicationPath(const std::filesystem::path& destination,
                             const std::filesystem::path& product) {
  if (!destination.is_absolute() || destination.native().rfind(L"\\\\", 0) == 0 ||
      SamePath(destination, destination.root_path()) ||
      !SamePath(destination, destination.lexically_normal()) ||
      Contains(destination, product) || Contains(product / L"Journal Broker", destination)) {
    throw std::runtime_error("Choose a separate local application directory");
  }
}

void ValidateApplicationDestination(const std::filesystem::path& destination,
                                    const std::optional<RegisteredProduct>& registered) {
  ValidateApplicationPath(destination, ProductDirectory());
  ValidateProtectedTree(destination);
  if (std::filesystem::exists(destination) && !std::filesystem::is_empty(destination) &&
      (!registered || !SamePath(destination, registered->application_directory))) {
    throw std::runtime_error("Choose an empty application directory or the registered Ame directory");
  }
}

}  // namespace ame::installer
