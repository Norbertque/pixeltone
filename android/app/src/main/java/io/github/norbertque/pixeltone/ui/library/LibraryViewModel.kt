package io.github.norbertque.pixeltone.ui.library

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.SearchResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.stateIn

class LibraryViewModel(app: Application) : AndroidViewModel(app) {
    private val library = (app as PixeltoneApp).library
    private val searchQuery = MutableStateFlow("")

    val index: StateFlow<LibraryIndex?> = library.index
    val query: StateFlow<String> = searchQuery.asStateFlow()

    /** The library narrowed to the search query; null until it has been read. */
    val results: StateFlow<SearchResult?> =
        combine(library.index, searchQuery) { index, query -> index?.search(query) }
            .flowOn(Dispatchers.Default)
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), null)

    fun search(query: String) {
        searchQuery.value = query
    }
}
