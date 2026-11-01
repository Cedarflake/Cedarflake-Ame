set(AME_CLASSIFICATION_VOCABULARY_DIR "${CMAKE_BINARY_DIR}/classification-vocabulary")
add_custom_target(ame_classification_vocabulary
  COMMAND "${AME_POWERSHELL}" -NoProfile -NonInteractive -ExecutionPolicy Bypass
    -File "${CMAKE_CURRENT_LIST_DIR}/../../tool/release_prepare_classification_vocabulary.ps1"
    -OutputDirectory "${AME_CLASSIFICATION_VOCABULARY_DIR}"
  VERBATIM
)
add_dependencies(${BINARY_NAME} ame_classification_vocabulary)
install(DIRECTORY "${AME_CLASSIFICATION_VOCABULARY_DIR}/"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}/classification-vocabulary"
  COMPONENT Runtime
)
